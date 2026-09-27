use crate::api::AppState;
use crate::display::DisplayStatus;
use crate::models::*;
use serde_json::{json, Value};
use tauri::{Manager, State, Window};

/// Get the current display status (mode, url, scheduler state, history).
#[tauri::command]
pub fn get_display_status(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<DisplayStatus, String> {
    controller.status()
}

/// Get the URL the player should show at startup.
#[tauri::command]
pub fn get_initial_url(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<Option<String>, String> {
    Ok(controller.initial_url())
}

/// Manually set the displayed URL.
#[tauri::command]
pub fn set_display_url(
    controller: State<'_, crate::display::DisplayController>,
    url: String,
) -> Result<(), String> {
    controller.set_url(&url)?;
    controller.emit_status();
    Ok(())
}

/// Show a stored page by id.
#[tauri::command]
pub fn show_display_page(
    controller: State<'_, crate::display::DisplayController>,
    page_id: i64,
) -> Result<(), String> {
    controller.show_page(page_id)?;
    controller.emit_status();
    Ok(())
}

/// Refresh the current page.
#[tauri::command]
pub fn refresh_display(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<(), String> {
    controller.refresh()?;
    controller.emit_status();
    Ok(())
}

/// Navigate back.
#[tauri::command]
pub fn display_back(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<Option<String>, String> {
    let u = controller.back()?;
    controller.emit_status();
    Ok(u)
}

/// Navigate forward.
#[tauri::command]
pub fn display_forward(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<Option<String>, String> {
    let u = controller.forward()?;
    controller.emit_status();
    Ok(u)
}

/// Clear the display so the player shows the default standby screen.
#[tauri::command]
pub fn reset_display(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<(), String> {
    controller.reset()?;
    Ok(())
}

/// Apply a temporary override.
#[tauri::command]
pub fn override_display(
    controller: State<'_, crate::display::DisplayController>,
    url: String,
    duration: Option<i64>,
    priority: Option<i64>,
) -> Result<(), String> {
    controller.override_page(&url, duration.unwrap_or(300), priority.unwrap_or(100))?;
    controller.emit_status();
    Ok(())
}

/// Called by the player view once a URL has loaded successfully.
#[tauri::command]
pub fn report_page_loaded(
    controller: State<'_, crate::display::DisplayController>,
    url: String,
) -> Result<(), String> {
    controller.report_loaded(&url);
    Ok(())
}

// ----- Pages -----

#[tauri::command]
pub fn list_pages(db: State<'_, crate::database::Database>) -> Result<Vec<Page>, String> {
    db.list_pages()
}

#[tauri::command]
pub fn get_page(
    db: State<'_, crate::database::Database>,
    id: i64,
) -> Result<Option<Page>, String> {
    db.get_page(id)
}

#[tauri::command]
pub fn create_page(
    controller: State<'_, crate::display::DisplayController>,
    page: NewPage,
) -> Result<Page, String> {
    let created = controller.db().create_page(&page)?;
    controller.emit_status();
    Ok(created)
}

#[tauri::command]
pub fn update_page(
    controller: State<'_, crate::display::DisplayController>,
    id: i64,
    page: UpdatePage,
) -> Result<Option<Page>, String> {
    let updated = controller.db().update_page(id, &page)?;
    controller.tick();
    Ok(updated)
}

#[tauri::command]
pub fn delete_page(
    controller: State<'_, crate::display::DisplayController>,
    id: i64,
) -> Result<bool, String> {
    let deleted = controller.db().delete_page(id)?;
    if deleted {
        controller.tick();
    }
    Ok(deleted)
}

// ----- Schedules -----

#[tauri::command]
pub fn list_schedules(db: State<'_, crate::database::Database>) -> Result<Vec<Schedule>, String> {
    db.list_schedules()
}

#[tauri::command]
pub fn create_schedule(
    controller: State<'_, crate::display::DisplayController>,
    schedule: NewSchedule,
) -> Result<Schedule, String> {
    let s = controller.db().create_schedule(&schedule)?;
    controller.tick();
    Ok(s)
}

#[tauri::command]
pub fn update_schedule(
    controller: State<'_, crate::display::DisplayController>,
    id: i64,
    schedule: UpdateSchedule,
) -> Result<Option<Schedule>, String> {
    let s = controller.db().update_schedule(id, &schedule)?;
    controller.tick();
    Ok(s)
}

#[tauri::command]
pub fn delete_schedule(
    controller: State<'_, crate::display::DisplayController>,
    id: i64,
) -> Result<bool, String> {
    let deleted = controller.db().delete_schedule(id)?;
    if deleted {
        controller.tick();
    }
    Ok(deleted)
}

// ----- Settings -----

#[tauri::command]
pub fn get_settings(db: State<'_, crate::database::Database>) -> Result<Value, String> {
    let all = db.all_settings()?;
    let mut map = serde_json::Map::new();
    for (k, v) in all {
        // Mask the API key for display but expose a boolean flag.
        if k == "api_key" {
            map.insert(
                k.clone(),
                json!({
                    "masked": !v.is_empty(),
                    "value": if v.is_empty() { String::new() } else { "*".repeat(12) },
                }),
            );
        } else {
            map.insert(k.clone(), Value::String(v));
        }
    }
    Ok(Value::Object(map))
}

/// Return the raw API key so the admin UI can display/copy it locally.
#[tauri::command]
pub fn get_api_key(db: State<'_, crate::database::Database>) -> Result<String, String> {
    Ok(db.get_setting_or("api_key", ""))
}

#[tauri::command]
pub fn update_settings(
    controller: State<'_, crate::display::DisplayController>,
    settings: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    for (k, v) in &settings {
        // Skip masked secrets sent back unchanged.
        if k == "api_key" && v.chars().all(|c| c == '*') {
            continue;
        }
        controller.db().set_setting(k, v)?;
    }
    Ok(())
}

/// Regenerate the API key.
#[tauri::command]
pub fn regenerate_api_key(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<String, String> {
    let key = uuid::Uuid::new_v4().simple().to_string();
    controller.db().set_setting("api_key", &key)?;
    Ok(key)
}

// ----- Device -----

#[tauri::command]
pub fn get_device(db: State<'_, crate::database::Database>) -> Result<Option<Device>, String> {
    db.get_device()
}

#[tauri::command]
pub fn update_device(
    db: State<'_, crate::database::Database>,
    device: UpdateDevice,
) -> Result<Option<Device>, String> {
    db.update_device(&device)
}

// ----- System -----

/// Restart the embedded HTTP server (used after changing port/bind settings).
#[tauri::command]
pub async fn restart_http_server(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>().inner().clone();
    let bind = state.db.get_setting_or("http_bind", "0.0.0.0");
    let port: u16 = state
        .db
        .get_setting_or("http_port", "8787")
        .parse()
        .unwrap_or(8787);
    let enabled = state.db.get_setting_or("http_enabled", "true") == "true";
    if !enabled {
        return Err("HTTP server is disabled".to_string());
    }
    crate::api::start_server(state, bind, port).await
}

/// Toggle the main window between admin (windowed) and player (fullscreen) mode.
#[tauri::command]
pub fn set_admin_mode(window: Window, enabled: bool) -> Result<(), String> {
    window.set_fullscreen(!enabled).map_err(|e| e.to_string())?;
    window
        .set_decorations(enabled)
        .map_err(|e| e.to_string())?;
    window
        .set_always_on_top(!enabled)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Run a scheduler tick on demand.
#[tauri::command]
pub fn scheduler_tick(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<(), String> {
    controller.tick();
    Ok(())
}

/// Current scheduler running state.
#[tauri::command]
pub fn scheduler_status(
    controller: State<'_, crate::display::DisplayController>,
) -> Result<bool, String> {
    Ok(controller.is_scheduler_running())
}

/// Enable or disable the scheduler loop.
#[tauri::command]
pub fn set_scheduler_enabled(
    controller: State<'_, crate::display::DisplayController>,
    enabled: bool,
) -> Result<(), String> {
    if enabled {
        controller.start_schedule();
    } else {
        controller.stop_schedule();
    }
    controller.emit_status();
    Ok(())
}

/// List recent command audit records.
#[tauri::command]
pub fn list_commands(
    db: State<'_, crate::database::Database>,
) -> Result<Vec<CommandRecord>, String> {
    db.list_commands(100)
}
