//! Scheduler + DisplayController integration tests.
//!
//! These exercise the pure scheduling precedence logic against an in-memory
//! database. They are compiled only under `cfg(test)`.

use crate::database::{now, Database};
use crate::models::*;
use chrono::{Duration, Utc};

use super::scheduler::{
    clear_pre_schedule, compute_target, save_pre_schedule, LAST_SCHEDULE_SETTING,
};

/// A schedule window that is never active: `start == end` makes the half-open
/// range `[start, end)` empty at every instant of the day.
fn never_active_schedule(db: &Database, page_id: i64) {
    db.create_schedule(&NewSchedule {
        page_id,
        schedule_type: SCHEDULE_TIME.to_string(),
        start_date: None,
        end_date: None,
        start_time: Some("00:00".to_string()),
        end_time: Some("00:00".to_string()),
        days: None,
        priority: None,
        enabled: true,
        sequence: None,
    })
    .unwrap();
}

fn page(db: &Database, name: &str, url: &str, duration: i64) -> i64 {
    db.create_page(&NewPage {
        name: name.to_string(),
        url: url.to_string(),
        duration,
        enabled: true,
    })
    .unwrap()
    .id
}

#[test]
fn falls_back_to_first_enabled_page() {
    let db = Database::in_memory().unwrap();
    let id = page(&db, "Second", "https://example.com/second", 30);
    let target = compute_target(&db).unwrap();
    // With no schedules, the first enabled page (the seed) is returned.
    assert!(target.url.contains("tauri.app") || target.page_id == Some(id));
    assert_eq!(target.mode, MODE_SCHEDULER);
}

#[test]
fn time_schedule_active_during_window() {
    let db = Database::in_memory().unwrap();
    let id = page(&db, "Day", "https://example.com/day", 60);
    // A window spanning the whole day should always be active.
    db.create_schedule(&NewSchedule {
        page_id: id,
        schedule_type: SCHEDULE_TIME.to_string(),
        start_date: None,
        end_date: None,
        start_time: Some("00:00".to_string()),
        end_time: Some("23:59".to_string()),
        days: None,
        priority: None,
        enabled: true,
        sequence: None,
    })
    .unwrap();

    let target = compute_target(&db).unwrap();
    assert_eq!(target.url, "https://example.com/day");
    assert_eq!(target.page_id, Some(id));
}

#[test]
fn higher_priority_time_schedule_wins() {
    let db = Database::in_memory().unwrap();
    let low = page(&db, "Low", "https://example.com/low", 60);
    let high = page(&db, "High", "https://example.com/high", 60);

    db.create_schedule(&NewSchedule {
        page_id: low,
        schedule_type: SCHEDULE_TIME.to_string(),
        start_date: None,
        end_date: None,
        start_time: Some("00:00".to_string()),
        end_time: Some("23:59".to_string()),
        days: None,
        priority: Some(1),
        enabled: true,
        sequence: None,
    })
    .unwrap();
    db.create_schedule(&NewSchedule {
        page_id: high,
        schedule_type: SCHEDULE_TIME.to_string(),
        start_date: None,
        end_date: None,
        start_time: Some("00:00".to_string()),
        end_time: Some("23:59".to_string()),
        days: None,
        priority: Some(100),
        enabled: true,
        sequence: None,
    })
    .unwrap();

    let target = compute_target(&db).unwrap();
    assert_eq!(target.page_id, Some(high));
}

#[test]
fn override_takes_precedence_then_expires() {
    let db = Database::in_memory().unwrap();
    let base = page(&db, "Base", "https://example.com/base", 60);
    db.create_schedule(&NewSchedule {
        page_id: base,
        schedule_type: SCHEDULE_TIME.to_string(),
        start_date: None,
        end_date: None,
        start_time: Some("00:00".to_string()),
        end_time: Some("23:59".to_string()),
        days: None,
        priority: None,
        enabled: true,
        sequence: None,
    })
    .unwrap();

    // Active override takes precedence.
    let future = Utc::now() + Duration::hours(1);
    db.set_display_state(&DisplayState {
        id: 1,
        current_page_id: None,
        current_url: Some("https://example.com/emergency".to_string()),
        mode: MODE_OVERRIDE.to_string(),
        started_at: Some(now()),
        expires_at: Some(future.to_rfc3339()),
    })
    .unwrap();
    let t = compute_target(&db).unwrap();
    assert_eq!(t.url, "https://example.com/emergency");
    assert_eq!(t.mode, MODE_OVERRIDE);

    // Expired override falls back to the scheduler.
    let past = Utc::now() - Duration::hours(1);
    db.set_display_state(&DisplayState {
        id: 1,
        current_page_id: None,
        current_url: Some("https://example.com/emergency".to_string()),
        mode: MODE_OVERRIDE.to_string(),
        started_at: Some(now()),
        expires_at: Some(past.to_rfc3339()),
    })
    .unwrap();
    let t = compute_target(&db).unwrap();
    assert_eq!(t.url, "https://example.com/base");
    assert_eq!(t.mode, MODE_SCHEDULER);
}

#[test]
fn manual_mode_not_overridden_without_active_schedule() {
    let db = Database::in_memory().unwrap();
    page(&db, "Seed", "https://example.com/seed", 60);

    // Operator sets a URL manually (admin UI / POST /api/v1/display).
    db.set_display_state(&DisplayState {
        id: 1,
        current_page_id: None,
        current_url: Some("https://example.com/manual".to_string()),
        mode: MODE_MANUAL.to_string(),
        started_at: Some(now()),
        expires_at: None,
    })
    .unwrap();

    // With no schedules configured, the manual URL must win over the
    // offline/first-page fallbacks on every scheduler tick.
    let target = compute_target(&db).unwrap();
    assert_eq!(target.url, "https://example.com/manual");
    assert_eq!(target.mode, MODE_MANUAL);
}

#[test]
fn idle_reset_shows_no_content() {
    let db = Database::in_memory().unwrap();
    page(&db, "Seed", "https://example.com/seed", 60);

    // Operator pressed Reset: display cleared, mode idle.
    db.set_display_state(&DisplayState {
        id: 1,
        current_page_id: None,
        current_url: None,
        mode: MODE_IDLE.to_string(),
        started_at: Some(now()),
        expires_at: None,
    })
    .unwrap();

    // No target => the player keeps showing the default standby screen
    // instead of falling back to the first enabled page.
    assert!(compute_target(&db).is_none());
}

#[test]
fn ended_time_schedule_restores_previous_page() {
    // Reproduces the reported bug: a page scheduled 12:10-12:12 must stop
    // showing once the window closes; the display returns to the page that was
    // showing before the schedule.
    let db = Database::in_memory().unwrap();
    let previous = page(&db, "Previous", "https://example.com/previous", 60);
    let scheduled = page(&db, "Scheduled", "https://example.com/scheduled", 60);
    never_active_schedule(&db, scheduled);

    // While the window was open the controller snapshotted the previous page,
    // the state is `scheduler`, and the scheduled page is also the "last
    // successful page" (the frontend reported it as loaded).
    save_pre_schedule(
        &db,
        "https://example.com/previous",
        Some(previous),
        MODE_MANUAL,
    );
    let _ = db.set_setting(LAST_SCHEDULE_SETTING, "https://example.com/scheduled");
    let _ = db.record_successful_page("https://example.com/scheduled");
    db.set_display_state(&DisplayState {
        id: 1,
        current_page_id: Some(scheduled),
        current_url: Some("https://example.com/scheduled".to_string()),
        mode: MODE_SCHEDULER.to_string(),
        started_at: Some(now()),
        expires_at: None,
    })
    .unwrap();

    let target = compute_target(&db).unwrap();
    assert_eq!(target.url, "https://example.com/previous");
    assert_eq!(target.page_id, Some(previous));
    assert_ne!(
        target.url, "https://example.com/scheduled",
        "the scheduled page must not linger after end_time"
    );
}

#[test]
fn ended_time_schedule_returns_to_standby_when_nothing_before() {
    // Nothing was displayed before the window (standby screen): when it ends
    // the display must clear again rather than keep showing the scheduled page.
    let db = Database::in_memory().unwrap();
    let scheduled = page(&db, "Scheduled", "https://example.com/scheduled", 60);
    never_active_schedule(&db, scheduled);

    save_pre_schedule(&db, "", None, MODE_IDLE);
    let _ = db.set_setting(LAST_SCHEDULE_SETTING, "https://example.com/scheduled");
    let _ = db.record_successful_page("https://example.com/scheduled");

    assert!(
        compute_target(&db).is_none(),
        "no content should remain once a window that started from standby ends"
    );
}

#[test]
fn cleared_snapshot_does_not_replay_ended_schedule_page() {
    // Without a snapshot the offline fallback must not resurrect the page
    // served by the schedule window that just ended.
    let db = Database::in_memory().unwrap();
    let other = page(&db, "Other", "https://example.com/other", 60);
    let scheduled = page(&db, "Scheduled", "https://example.com/scheduled", 60);
    never_active_schedule(&db, scheduled);

    clear_pre_schedule(&db);
    let _ = db.set_setting(LAST_SCHEDULE_SETTING, "https://example.com/scheduled");
    let _ = db.record_successful_page("https://example.com/scheduled");

    let target = compute_target(&db).unwrap();
    // The ended schedule's URL is rejected; the display falls back to the
    // regular content (the seeded first enabled page here).
    assert_ne!(target.url, "https://example.com/scheduled");
    assert_ne!(target.page_id, Some(scheduled));
    let _ = other;
}

#[test]
fn rotation_cycles_by_duration() {
    let db = Database::in_memory().unwrap();
    let a = page(&db, "A", "https://example.com/a", 10);
    let b = page(&db, "B", "https://example.com/b", 10);

    db.create_schedule(&NewSchedule {
        page_id: a,
        schedule_type: SCHEDULE_ROTATION.to_string(),
        start_date: None,
        end_date: None,
        start_time: None,
        end_time: None,
        days: None,
        priority: None,
        enabled: true,
        sequence: Some(1),
    })
    .unwrap();
    db.create_schedule(&NewSchedule {
        page_id: b,
        schedule_type: SCHEDULE_ROTATION.to_string(),
        start_date: None,
        end_date: None,
        start_time: None,
        end_time: None,
        days: None,
        priority: None,
        enabled: true,
        sequence: Some(2),
    })
    .unwrap();

    let target = compute_target(&db).unwrap();
    assert!(
        target.url == "https://example.com/a" || target.url == "https://example.com/b"
    );
    assert_eq!(target.mode, MODE_SCHEDULER);
}