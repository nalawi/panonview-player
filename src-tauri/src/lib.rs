mod api;

// Screen capture memakai `xcap` yang hanya mendukung desktop. Di mobile dipakai
// stub dengan signature yang sama (capture_png & capture_data_url) agar kode
// pemanggil (commands.rs dan api/capture.rs) tetap bisa dikompilasi.
#[cfg(desktop)]
mod capture;
#[cfg(not(desktop))]
mod capture {
    const UNSUPPORTED: &str = "Screen capture is not supported on this platform";

    pub fn capture_png() -> Result<Vec<u8>, String> {
        Err(UNSUPPORTED.to_string())
    }

    pub fn capture_data_url() -> Result<String, String> {
        Err(UNSUPPORTED.to_string())
    }
}
mod commands;
mod database;
mod display;
mod models;

use api::AppState;
use database::Database;
use display::DisplayController;
use std::sync::Mutex;
use tauri::Manager;

// ---------------------------------------------------------------------------
// Embedded UI (Android only)
// ---------------------------------------------------------------------------
// Di Android, resource Tauri berada di dalam APK sehingga tidak bisa dibaca
// lewat std::fs. Karena itu `dist/` di-embed ke binary Rust, lalu diekstrak ke
// app_data_dir/ui saat startup. Di desktop tidak dipakai (resource_dir cukup),
// sehingga `tauri dev` tidak mensyaratkan folder `dist/` ada saat kompilasi.
#[cfg(target_os = "android")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../dist/"]
struct UiAssets;

/// Ekstrak aset embedded ke `<app_data_dir>/ui`. Selalu dibersihkan dulu agar
/// isinya sinkron dengan versi APK yang terpasang.
#[cfg(target_os = "android")]
fn extract_embedded_ui(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let base = app.path().app_data_dir().ok()?.join("ui");
    let _ = std::fs::remove_dir_all(&base);
    for name in UiAssets::iter() {
        let file = match UiAssets::get(&name) {
            Some(f) => f,
            None => continue,
        };
        let dest = base.join(name.as_ref());
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[ui] cannot create {}: {}", parent.display(), e);
                return None;
            }
        }
        if let Err(e) = std::fs::write(&dest, file.data.as_ref()) {
            eprintln!("[ui] cannot write {}: {}", dest.display(), e);
            return None;
        }
    }
    if base.join("index.html").exists() {
        Some(base)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// HTTP server handle (supaya bisa di-restart)
// ---------------------------------------------------------------------------

/// Menyimpan task server HTTP yang sedang berjalan agar bisa dihentikan
/// (abort) sebelum dijalankan ulang dengan port/bind baru.
pub struct HttpServerHandle(pub Mutex<Option<tauri::async_runtime::JoinHandle<()>>>);

/// Jalankan server HTTP sebagai background task.
pub fn spawn_http_server(
    state: AppState,
    bind: String,
    port: u16,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        println!("[http] starting on {}:{}", bind, port);
        if let Err(e) = api::start_server(state, bind.clone(), port).await {
            eprintln!("[http] server stopped / failed to start on {}:{}: {}", bind, port, e);
        }
    })
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

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

/// Android: aset dibaca dari binary (embedded) dan diekstrak ke app_data_dir.
#[cfg(target_os = "android")]
fn resolve_ui_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    extract_embedded_ui(app)
}

/// Desktop: locate the built admin web UI to serve over HTTP at `/ui`.
///
/// Checks, in order:
///   1. The bundled resource directory (`<resource_dir>/ui/index.html`).
///   2. The `frontendDist` configuration from tauri.conf.json (dev only).
///   3. Common development `dist` locations relative to the CWD.
#[cfg(not(target_os = "android"))]
fn resolve_ui_dir(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TauriConf {
        #[serde(rename = "frontendDist")]
        frontend_dist: Option<String>,
    }

    let has_index = |p: &std::path::Path| p.join("index.html").exists();

    // 1. Bundled resource directory
    if let Ok(res) = app.path().resource_dir() {
        let p = res.join("ui");
        if has_index(&p) {
            return Some(p);
        }
    }

    // 2. frontendDist from tauri.conf.json (dev only)
    if let Ok(conf_str) = std::fs::read_to_string("tauri.conf.json") {
        if let Ok(conf) = serde_json::from_str::<TauriConf>(&conf_str) {
            if let Some(frontend_dist) = conf.frontend_dist {
                if let Ok(cwd) = std::env::current_dir() {
                    let resolved = cwd.join(frontend_dist);
                    if has_index(&resolved) {
                        return Some(resolved);
                    }
                }
            }
        }
    }

    // 3. Common development dist locations relative to the CWD
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

/// Android: tahan layar tetap menyala selama app di foreground
/// (FLAG_KEEP_SCREEN_ON pada window Activity). Dipanggil via JNI di UI thread.
#[cfg(target_os = "android")]
fn keep_screen_on(window: &tauri::WebviewWindow, enable: bool) {
    let res = window.with_webview(move |webview| {
        webview.jni_handle().exec(move |env, activity, _webview| {
            use jni::objects::JValue;
            // WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
            const FLAG_KEEP_SCREEN_ON: i32 = 0x0000_0080;

            let win = match env
                .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
                .and_then(|v| v.l())
            {
                Ok(w) => w,
                Err(e) => {
                    let _ = env.exception_clear();
                    eprintln!("[android] getWindow failed: {e}");
                    return;
                }
            };
            let method = if enable { "addFlags" } else { "clearFlags" };
            if let Err(e) = env.call_method(&win, method, "(I)V", &[JValue::Int(FLAG_KEEP_SCREEN_ON)]) {
                let _ = env.exception_clear();
                eprintln!("[android] {method} failed: {e}");
            } else {
                println!("[android] keep screen on = {enable}");
            }
        });
    });
    if let Err(e) = res {
        eprintln!("[android] with_webview failed: {e}");
    }
}

/// Configure the main window for kiosk/player mode.
///
/// Window-control methods only exist on desktop targets (`#[cfg(desktop)]`).
/// On mobile this is a no-op.
fn apply_window_mode(window: &tauri::WebviewWindow, player_mode: bool) {
    #[cfg(desktop)]
    {
        let _ = window.set_fullscreen(player_mode);
        let _ = window.set_resizable(!player_mode);
        let _ = window.set_decorations(!player_mode);
        let _ = window.set_always_on_top(player_mode);
    }
    #[cfg(not(desktop))]
    {
        let _ = (window, player_mode);
    }
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

                // Android: jaga layar tetap menyala (setting `keep_screen_on`, default true).
                #[cfg(target_os = "android")]
                {
                    if db.get_setting_or("keep_screen_on", "true") == "true" {
                        keep_screen_on(&window, true);
                    }
                }
            }

            // 5. Start the embedded HTTP server (unless disabled).
            let http_enabled = db.get_setting_or("http_enabled", "true") == "true";
            let server_handle = if http_enabled {
                let bind = db.get_setting_or("http_bind", "0.0.0.0");
                let port: u16 = db
                    .get_setting_or("http_port", "8787")
                    .parse()
                    .unwrap_or(8787);
                Some(spawn_http_server(state.clone(), bind, port))
            } else {
                println!("[http] server disabled by settings");
                None
            };
            app.manage(HttpServerHandle(Mutex::new(server_handle)));

            // 6. Start the scheduler loop; it also applies the initial target.
            controller.start_schedule();

            Ok(())
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
            commands::capture_screen,
            commands::get_local_ip,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}