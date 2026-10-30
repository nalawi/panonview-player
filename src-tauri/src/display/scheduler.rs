use crate::database::Database;
use crate::models::*;
use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Timelike, Utc, Weekday};

/// Settings key holding the page that was on screen before a schedule took
/// over, so the display can return to it once the schedule window closes.
pub const PRE_SCHEDULE_SETTING: &str = "pre_schedule_display";
/// Settings key holding the operator-configured standby ("player default")
/// page. Empty means "show the standby screen".
pub const DEFAULT_PAGE_SETTING: &str = "default_page_id";
/// Settings key holding the URL of the page served by the schedule window that
/// most recently took over. The offline fallback must never replay it once the
/// window has closed.
pub const LAST_SCHEDULE_SETTING: &str = "last_schedule_url";

/// Why the scheduler chose a target. The controller uses this to decide when
/// to snapshot the pre-schedule page and when to drop the snapshot again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSource {
    /// Temporary operator override.
    Override,
    /// An active time-based schedule.
    Schedule,
    /// The rotation cycle.
    Rotation,
    /// Operator-set URL / page (manual mode).
    Manual,
    /// The page that was on screen before the schedule started.
    Restore,
    /// The operator-configured default page.
    Default,
    /// Offline / bootstrap fallbacks.
    Fallback,
}

/// The desired display target computed by the scheduler.
#[derive(Debug, Clone)]
pub struct Target {
    pub page_id: Option<i64>,
    pub url: String,
    pub mode: String,
    /// When set, the target is a temporary override that should expire.
    pub expires_at: Option<String>,
    /// Why this target was chosen.
    pub source: TargetSource,
}

/// Snapshot of what was on screen before a schedule took over.
///
/// `standby` is true when nothing was displayed (the player standby screen).
/// In that case the display must be cleared again once the window closes,
/// instead of resurrecting the just-ended (or any other) page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreSchedule {
    pub url: String,
    pub page_id: Option<i64>,
    pub mode: String,
    pub standby: bool,
}

/// Persist what is about to be replaced by a schedule. `url` may be empty to
/// record that the standby screen was showing, so the display returns to
/// standby once the schedule window closes.
pub fn save_pre_schedule(db: &Database, url: &str, page_id: Option<i64>, mode: &str) {
    let payload = serde_json::json!({
        "standby": url.is_empty(),
        "url": url,
        "page_id": page_id,
        "mode": mode,
    })
    .to_string();
    let _ = db.set_setting(PRE_SCHEDULE_SETTING, &payload);
}

/// Read the snapshot without consuming it (safe to call from pure code paths).
pub fn peek_pre_schedule(db: &Database) -> Option<PreSchedule> {
    let raw = db.get_setting(PRE_SCHEDULE_SETTING).ok().flatten()?;
    if raw.trim().is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let standby = value
        .get("standby")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);
    let url = value
        .get("url")
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();
    // Only meaningful when it recorded a real page or an explicit standby.
    if url.is_empty() && !standby {
        return None;
    }
    Some(PreSchedule {
        url,
        page_id: value.get("page_id").and_then(|p| p.as_i64()),
        mode: value
            .get("mode")
            .and_then(|m| m.as_str())
            .unwrap_or(MODE_MANUAL)
            .to_string(),
        standby,
    })
}

/// Forget the snapshot once it has been restored (or superseded).
pub fn clear_pre_schedule(db: &Database) {
    let _ = db.set_setting(PRE_SCHEDULE_SETTING, "");
}

/// Resolve the operator-configured default page, if it still exists and is
/// enabled. An empty/invalid setting means "no default page".
fn default_page_target(db: &Database) -> Option<Target> {
    let raw = db.get_setting(DEFAULT_PAGE_SETTING).ok().flatten()?;
    let id: i64 = raw.trim().parse().ok()?;
    let page = db.get_page(id).ok().flatten()?;
    if !page.enabled {
        return None;
    }
    Some(Target {
        page_id: Some(page.id),
        url: page.url,
        // Restore in manual mode so the choice survives subsequent scheduler
        // ticks instead of being recomputed (and possibly cleared) each time.
        mode: MODE_MANUAL.to_string(),
        expires_at: None,
        source: TargetSource::Default,
    })
}

/// Parse "HH:MM" or "HH:MM:SS" into a NaiveTime.
fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .ok()
        .or_else(|| NaiveTime::parse_from_str(s, "%H:%M:%S").ok())
}

/// Map a chrono Weekday to a 3-letter uppercase day code.
fn day_code(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "MON",
        Weekday::Tue => "TUE",
        Weekday::Wed => "WED",
        Weekday::Thu => "THU",
        Weekday::Fri => "FRI",
        Weekday::Sat => "SAT",
        Weekday::Sun => "SUN",
    }
}

/// Parse an RFC3339 timestamp if possible.
fn parse_dt(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Determine whether a time-based schedule is active right now.
fn time_schedule_active(s: &Schedule, now_local: DateTime<Local>) -> bool {
    if !s.enabled {
        return false;
    }
    if s.schedule_type != SCHEDULE_TIME {
        return false;
    }

    // Optional date window (YYYY-MM-DD).
    let today = now_local.date_naive();
    if let Some(start) = &s.start_date {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(start, "%Y-%m-%d") {
            if today < d {
                return false;
            }
        }
    }
    if let Some(end) = &s.end_date {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(end, "%Y-%m-%d") {
            if today > d {
                return false;
            }
        }
    }

    // Day-of-week filter.
    if let Some(days) = &s.days {
        let code = day_code(now_local.weekday());
        let allowed = days
            .split(',')
            .map(|d| d.trim().to_uppercase())
            .any(|d| d == code);
        if !allowed {
            return false;
        }
    }

    // Time-of-day window.
    let (start_time, end_time) = match (&s.start_time, &s.end_time) {
        (Some(a), Some(b)) => (parse_time(a), parse_time(b)),
        _ => return true, // no time window means always active within date/day filter
    };
    let (start_time, end_time) = match (start_time, end_time) {
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };

    let current = now_local.time();
    if start_time <= end_time {
        current >= start_time && current < end_time
    } else {
        // Window spans midnight.
        current >= start_time || current < end_time
    }
}

/// Compute the target for rotation schedules. Rotation cycles through all
/// enabled rotation schedules ordered by `sequence` (then id), each shown for
/// its page duration.
fn rotation_target(db: &Database, schedules: &[Schedule]) -> Option<Target> {
    let mut rot: Vec<&Schedule> = schedules
        .iter()
        .filter(|s| s.enabled && s.schedule_type == SCHEDULE_ROTATION)
        .collect();
    if rot.is_empty() {
        return None;
    }
    rot.sort_by_key(|s| (s.sequence.unwrap_or(i64::MAX), s.id));

    // Resolve pages once.
    let mut pages: Vec<(Page, &Schedule)> = Vec::new();
    for s in rot {
        if let Ok(Some(p)) = db.get_page(s.page_id) {
            if p.enabled {
                pages.push((p, s));
            }
        }
    }
    if pages.is_empty() {
        return None;
    }

    // Anchor the cycle at midnight local time so the rotation is deterministic
    // and resumes consistently after a restart.
    let midnight = Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|naive| {
            naive
                .and_local_timezone(Local)
                .single()
                .unwrap_or_else(Local::now)
        })
        .unwrap_or_else(Local::now);

    let elapsed = (Local::now() - midnight).num_seconds().max(0) as i64;
    let total: i64 = pages.iter().map(|(p, _)| p.duration.max(1)).sum();
    if total <= 0 {
        return None;
    }
    let pos = elapsed % total;

    let mut acc = 0i64;
    for (p, _s) in &pages {
        acc += p.duration.max(1);
        if pos < acc {
            return Some(Target {
                page_id: Some(p.id),
                url: p.url.clone(),
                mode: MODE_SCHEDULER.to_string(),
                expires_at: None,
                source: TargetSource::Rotation,
            });
        }
    }

    let (p, _) = &pages[0];
    Some(Target {
        page_id: Some(p.id),
        url: p.url.clone(),
        mode: MODE_SCHEDULER.to_string(),
        expires_at: None,
        source: TargetSource::Rotation,
    })
}

/// Compute the desired target based on current schedules and persisted state.
///
/// Precedence:
///   1. Active override (highest priority, until it expires)
///   2. Active time-based schedule (highest priority wins)
///   3. Rotation schedule
///   4. Idle (display reset): no content until something provides it
///   5. Manual mode: keep the operator-chosen URL when no schedule is active
///   6. Restore the page that was on screen before the schedule started
///   7. Operator-configured default page
///   8. Last successful page (offline fallback), else first enabled page
pub fn compute_target(db: &Database) -> Option<Target> {
    let now_utc = Utc::now();
    let now_local = Local::now();
    let state = db.get_display_state().unwrap_or_default();

    // 1. Override.
    if state.mode == MODE_OVERRIDE {
        if let (Some(url), Some(exp)) = (&state.current_url, &state.expires_at) {
            if let Some(exp_dt) = parse_dt(exp) {
                if exp_dt > now_utc {
                    return Some(Target {
                        page_id: state.current_page_id,
                        url: url.clone(),
                        mode: MODE_OVERRIDE.to_string(),
                        expires_at: state.expires_at.clone(),
                        source: TargetSource::Override,
                    });
                }
            } else {
                // Malformed expiry — treat as still active to be safe.
                return Some(Target {
                    page_id: state.current_page_id,
                    url: url.clone(),
                    mode: MODE_OVERRIDE.to_string(),
                    expires_at: state.expires_at.clone(),
                    source: TargetSource::Override,
                });
            }
        }
        // Override expired — fall through to scheduler.
    }

    let schedules = db.list_schedules().unwrap_or_default();

    // 2. Time-based schedules (highest priority first).
    let mut active_time: Vec<&Schedule> = schedules
        .iter()
        .filter(|s| time_schedule_active(s, now_local))
        .collect();
    if !active_time.is_empty() {
        active_time.sort_by(|a, b| b.priority.cmp(&a.priority));
        if let Some(s) = active_time.first() {
            if let Ok(Some(p)) = db.get_page(s.page_id) {
                if p.enabled {
                    return Some(Target {
                        page_id: Some(p.id),
                        url: p.url,
                        mode: MODE_SCHEDULER.to_string(),
                        expires_at: None,
                        source: TargetSource::Schedule,
                    });
                }
            }
        }
    }

    // 3. Rotation schedules.
    if let Some(t) = rotation_target(db, &schedules) {
        return Some(t);
    }

    // 4. Idle: the operator reset the display ("Reset" in the dashboard) and
    // wants the default standby screen. Return no target so the fallbacks
    // below do not repopulate the display; an active schedule or override
    // (handled above) still wins, and setting a URL leaves idle mode.
    if state.mode == MODE_IDLE {
        return None;
    }

    // 5. Manual mode: a URL set by the operator (admin UI or
    // POST /api/v1/display) must survive scheduler ticks when no schedule is
    // actually active. Without this, the offline fallbacks below would flip
    // the display back to the last successful / first enabled page on every
    // tick and the manual choice would be overridden by "scheduler" mode.
    if state.mode == MODE_MANUAL {
        if let Some(url) = &state.current_url {
            if !url.is_empty() {
                return Some(Target {
                    page_id: state.current_page_id,
                    url: url.clone(),
                    mode: MODE_MANUAL.to_string(),
                    expires_at: None,
                    source: TargetSource::Manual,
                });
            }
        }
    }

    // 6. Restore the page that was on screen before the schedule started.
    // This is what makes an expired time window fall back to the previous
    // content instead of replaying the just-ended scheduled page (which is
    // also the "last successful page" the controller persisted).
    if let Some(pre) = peek_pre_schedule(db) {
        if pre.standby || pre.url.is_empty() {
            // Nothing was displayed before the schedule: return to standby
            // rather than showing the just-ended (or any other) page.
            return None;
        }
        return Some(Target {
            page_id: pre.page_id,
            url: pre.url,
            mode: pre.mode,
            expires_at: None,
            source: TargetSource::Restore,
        });
    }

    // 7. Operator-configured default page.
    if let Some(t) = default_page_target(db) {
        return Some(t);
    }

    // 8. Offline fallback: last successful page — but never the page from a
    // schedule window that just ended, otherwise the display would keep the
    // scheduled page instead of returning to the previous/default content.
    let (last_url, _) = db.last_successful();
    let last_scheduled_url = db
        .get_setting(LAST_SCHEDULE_SETTING)
        .ok()
        .flatten()
        .unwrap_or_default();
    if let Some(url) = last_url {
        if !url.is_empty() && url != last_scheduled_url {
            return Some(Target {
                page_id: None,
                url,
                mode: MODE_SCHEDULER.to_string(),
                expires_at: None,
                source: TargetSource::Fallback,
            });
        }
    }

    // 8b. First enabled page.
    if let Ok(pages) = db.list_pages() {
        if let Some(p) = pages.into_iter().find(|p| p.enabled) {
            return Some(Target {
                page_id: Some(p.id),
                url: p.url,
                mode: MODE_SCHEDULER.to_string(),
                expires_at: None,
                source: TargetSource::Fallback,
            });
        }
    }

    None
}

/// Compute the number of seconds until the next scheduler transition, so the
/// scheduler loop can sleep efficiently.
pub fn seconds_until_next_transition(db: &Database) -> i64 {
    let now_local = Local::now();
    let now_utc = Utc::now();
    let state = db.get_display_state().unwrap_or_default();
    let mut min_secs = 60i64;

    // Override expiry.
    if state.mode == MODE_OVERRIDE {
        if let Some(exp) = &state.expires_at {
            if let Some(exp_dt) = parse_dt(exp) {
                let secs = (exp_dt - now_utc).num_seconds();
                if secs > 0 {
                    min_secs = min_secs.min(secs);
                }
            }
        }
    }

    let schedules = db.list_schedules().unwrap_or_default();
    for s in &schedules {
        if !s.enabled || s.schedule_type != SCHEDULE_TIME {
            continue;
        }
        // Only consider schedules that could become active today/now.
        if let Some(days) = &s.days {
            let code = day_code(now_local.weekday());
            if !days.split(',').any(|d| d.trim().to_uppercase() == code) {
                continue;
            }
        }
        for t in [&s.start_time, &s.end_time].into_iter().flatten() {
            if let Some(nt) = parse_time(t) {
                let today_target = now_local
                    .date_naive()
                    .and_hms_opt(nt.hour(), nt.minute(), nt.second());
                if let Some(target_naive) = today_target {
                    if let Some(target) = target_naive.and_local_timezone(Local).single() {
                        let mut delta = target - now_local;
                        if delta < Duration::zero() {
                            delta += Duration::days(1);
                        }
                        let secs = delta.num_seconds();
                        if secs > 0 {
                            min_secs = min_secs.min(secs);
                        }
                    }
                }
            }
        }
    }

    // Rotation: never sleep longer than the shortest rotation page duration.
    let rotation_min = schedules
        .iter()
        .filter(|s| s.enabled && s.schedule_type == SCHEDULE_ROTATION)
        .filter_map(|s| db.get_page(s.page_id).ok().flatten())
        .map(|p| p.duration.max(1))
        .min();
    if let Some(d) = rotation_min {
        min_secs = min_secs.min(d);
    }

    min_secs.max(1)
}