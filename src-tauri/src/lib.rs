mod api;
mod commands;
mod database;
mod display;
mod models;

use api::AppState;
use database::Database;
use display::DisplayController;
use tauri::{Manager, WindowEvent};

/// Resolve the on-disk location for the SQLite database, creating the parent
/// directory if needed.
fn resolve_db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("cannot resolve app data dir: {}", e))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("web_player.db"))
}

/// Locate the built admin web UI to serve over HTTP at `/ui`.
///
/// Checks, in order:
///   1. The bundled resource directory (`<resource_dir>/ui/index.html`).
///   2. Common development `dist` locations relative to the CWD.
fn resolve_ui_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let has_index = |p: &std::path::Path| p.join("index.html").exists();

    if let Ok(res) = app.path().resource_dir() {
        let p = res.join("ui");
        if has_index(&p) {
            return Some(p);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            cwd.join("dist"),             // run from apps/player
            cwd.join("../dist"),          // run from apps/player/src-tauri
            cwd.join("apps/player/dist"), // run from repo root
        ];
        for c in candidates {
            if has_index(&c) {
                return Some(c);
            }
        }
    }

    None
}

/// Configure the main window for kiosk/player mode.
fn apply_window_mode(window: &tauri::WebviewWindow, player_mode: bool) {
    let _ = window.set_fullscreen(player_mode);
    let _ = window.set_decorations(!player_mode);
    let _ = window.set_always_on_top(player_mode);
    let _ = window.set_resizable(!player_mode);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 1. Database.
            let db_path = resolve_db_path(app.handle())
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let db = Database::new(&db_path)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            // 2. DisplayController (single source of truth).
            let controller = DisplayController::new(db.clone());
            controller.attach_app(app.handle().clone());

            // 3. Shared state for commands + HTTP server.
            let ui_dir = resolve_ui_dir(app.handle());
            match &ui_dir {
                Some(p) => println!("[ui] serving admin web UI from {}", p.display()),
                None => println!("[ui] no built UI found; API only"),
            }
            let state = AppState {
                controller: controller.clone(),
                db: db.clone(),
                ui_dir,
            };
            app.manage(state.clone());

            // 4. Window mode based on settings (defaults to player/fullscreen).
            if let Some(window) = app.get_webview_window("main") {
                let startup_mode = db.get_setting_or("startup_mode", "player");
                apply_window_mode(&window, startup_mode != "admin");
            }

            // 5. Start the embedded HTTP server (unless disabled).
            let http_enabled = db.get_setting_or("http_enabled", "true") == "true";
            if http_enabled {
                let bind = db.get_setting_or("http_bind", "0.0.0.0");
                let port: u16 = db
                    .get_setting_or("http_port", "8787")
                    .parse()
                    .unwrap_or(8787);
                let server_state = state.clone();
                tauri::async_runtime::spawn(async move {
                    match api::start_server(server_state, bind.clone(), port).await {
                        Ok(_) => println!("[http] listening on {}:{}", bind, port),
                        Err(e) => eprintln!("[http] failed to start: {}", e),
                    }
                });
            } else {
                println!("[http] server disabled by settings");
            }

            // 6. Start the scheduler loop; it also applies the initial target.
            controller.start_schedule();

            Ok(())
        })
        .on_window_event(|window, event| {
            // Re-assert fullscreen if the OS/user tries to leave kiosk mode
            // while in player mode. Admin mode keeps normal window behavior.
            if let WindowEvent::CloseRequested { .. } = event {
                let _ = window;
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_display_status,
            commands::get_initial_url,
            commands::set_display_url,
            commands::show_display_page,
            commands::refresh_display,
            commands::display_back,
            commands::display_forward,
            commands::reset_display,
            commands::override_display,
            commands::report_page_loaded,
            commands::list_pages,
            commands::get_page,
            commands::create_page,
            commands::update_page,
            commands::delete_page,
            commands::list_schedules,
            commands::create_schedule,
            commands::update_schedule,
            commands::delete_schedule,
            commands::get_settings,
            commands::get_api_key,
            commands::update_settings,
            commands::regenerate_api_key,
            commands::get_device,
            commands::update_device,
            commands::restart_http_server,
            commands::set_admin_mode,
            commands::scheduler_tick,
            commands::scheduler_status,
            commands::set_scheduler_enabled,
            commands::list_commands,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}