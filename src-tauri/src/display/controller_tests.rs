//! DisplayController restart/persistence tests.
//!
//! They exercise the "remember the last active page across a stop/restart"
//! behavior against a real (temporary) on-disk database: each test simulates
//! an app shutdown by dropping the controller + database and reopening the
//! same file, exactly like a process restart would.

use crate::database::{now, Database};
use crate::display::DisplayController;
use crate::models::*;

#[test]
fn last_active_url_survives_restart() {
    let dir = std::env::temp_dir().join(format!("panonview_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("web_player.db");

    // First run: operator points the player at a URL.
    let db = Database::new(&path).unwrap();
    let controller = DisplayController::new(db.clone());
    controller
        .set_url("https://example.com/last-active")
        .unwrap();
    assert_eq!(
        controller.initial_url().as_deref(),
        Some("https://example.com/last-active")
    );
    drop(controller);
    drop(db);

    // "Restart": reopen the same file with a brand new controller.
    let db2 = Database::new(&path).unwrap();
    let controller2 = DisplayController::new(db2);
    assert_eq!(
        controller2.initial_url().as_deref(),
        Some("https://example.com/last-active"),
        "the last active URL must be remembered across a restart"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn shown_page_survives_restart() {
    let dir = std::env::temp_dir().join(format!("panonview_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("web_player.db");

    let db = Database::new(&path).unwrap();
    let page = db
        .create_page(&NewPage {
            name: "Lobby".to_string(),
            url: "https://example.com/lobby".to_string(),
            duration: 60,
            enabled: true,
        })
        .unwrap();
    let controller = DisplayController::new(db.clone());
    controller.show_page(page.id).unwrap();
    drop(controller);
    drop(db);

    let db2 = Database::new(&path).unwrap();
    let controller2 = DisplayController::new(db2);
    assert_eq!(
        controller2.initial_url().as_deref(),
        Some("https://example.com/lobby"),
        "a page shown before shutdown must still be showing after restart"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn navigation_history_survives_restart() {
    let dir = std::env::temp_dir().join(format!("panonview_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("web_player.db");

    let db = Database::new(&path).unwrap();
    let controller = DisplayController::new(db.clone());
    controller.set_url("https://example.com/first").unwrap();
    controller.set_url("https://example.com/second").unwrap();
    drop(controller);
    drop(db);

    let db2 = Database::new(&path).unwrap();
    let controller2 = DisplayController::new(db2);

    let status = controller2.status().unwrap();
    assert!(
        status.can_back,
        "back navigation must still be available after a restart"
    );
    assert_eq!(
        status.current_url.as_deref(),
        Some("https://example.com/second")
    );
    assert_eq!(
        controller2.back().unwrap().as_deref(),
        Some("https://example.com/first")
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reset_stays_standby_after_restart() {
    let dir = std::env::temp_dir().join(format!("panonview_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("web_player.db");

    let db = Database::new(&path).unwrap();
    let controller = DisplayController::new(db.clone());
    controller.set_url("https://example.com/somewhere").unwrap();
    controller.reset().unwrap();
    drop(controller);
    drop(db);

    let db2 = Database::new(&path).unwrap();
    let controller2 = DisplayController::new(db2);
    assert_eq!(
        controller2.initial_url(),
        None,
        "an operator reset must be remembered as 'standby' across a restart"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn expired_override_is_not_restored() {
    let dir = std::env::temp_dir().join(format!("panonview_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("web_player.db");

    let db = Database::new(&path).unwrap();
    // An override whose expiry passed while the app was stopped.
    let past = chrono::Utc::now() - chrono::Duration::hours(1);
    db.set_display_state(&DisplayState {
        id: 1,
        current_page_id: None,
        current_url: Some("https://example.com/emergency".to_string()),
        mode: MODE_OVERRIDE.to_string(),
        started_at: Some(now()),
        expires_at: Some(past.to_rfc3339()),
    })
    .unwrap();
    drop(db);

    let db2 = Database::new(&path).unwrap();
    let controller = DisplayController::new(db2);
    assert_ne!(
        controller.initial_url().as_deref(),
        Some("https://example.com/emergency"),
        "an override that expired while the app was stopped must not be restored"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
