use crate::database::Database;
use crate::models::*;
use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, Timelike, Utc, Weekday};

/// The desired display target computed by the scheduler.
#[derive(Debug, Clone)]
pub struct Target {
    pub page_id: Option<i64>,
    pub url: String,
    pub mode: String,
    /// When set, the target is a temporary override that should expire.
    pub expires_at: Option<String>,
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
            });
        }
    }

    let (p, _) = &pages[0];
    Some(Target {
        page_id: Some(p.id),
        url: p.url.clone(),
        mode: MODE_SCHEDULER.to_string(),
        expires_at: None,
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
///   6. Last successful page (offline fallback), else first enabled page
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
                    });
                }
            } else {
                // Malformed expiry — treat as still active to be safe.
                return Some(Target {
                    page_id: state.current_page_id,
                    url: url.clone(),
                    mode: MODE_OVERRIDE.to_string(),
                    expires_at: state.expires_at.clone(),
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
                });
            }
        }
    }

    // 6. Offline fallback: last successful page.
    let (last_url, _) = db.last_successful();
    if let Some(url) = last_url {
        return Some(Target {
            page_id: None,
            url,
            mode: MODE_SCHEDULER.to_string(),
            expires_at: None,
        });
    }

    // 5. First enabled page.
    if let Ok(pages) = db.list_pages() {
        if let Some(p) = pages.into_iter().find(|p| p.enabled) {
            return Some(Target {
                page_id: Some(p.id),
                url: p.url,
                mode: MODE_SCHEDULER.to_string(),
                expires_at: None,
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
