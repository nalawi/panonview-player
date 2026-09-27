use crate::api::{success, ApiError, ApiResult, AppState};
use crate::database::now;
use crate::models::UpdateDevice;
use axum::extract::State;
use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub device_id: String,
    pub device_name: String,
    pub hostname: String,
    pub platform: String,
    pub version: String,
    pub location: String,
    pub group: String,
    pub current_url: Option<String>,
    pub mode: String,
    pub online: bool,
    pub scheduler_running: bool,
    pub uptime_started_at: Option<String>,
    pub last_successful_url: Option<String>,
    pub last_successful_at: Option<String>,
}

/// `GET /api/v1/status` — device + display status heartbeat.
pub async fn get_status(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let device = state.db.get_device().map_err(ApiError::internal)?;
    let display = state.controller.status().map_err(ApiError::internal)?;

    let (device_id, device_name, hostname, platform, version, location, group) = match device {
        Some(d) => (
            d.id,
            d.name,
            d.hostname.unwrap_or_default(),
            d.platform.unwrap_or_default(),
            d.version.unwrap_or_default(),
            d.location.unwrap_or_default(),
            d.group_name.unwrap_or_default(),
        ),
        None => (
            String::new(),
            String::new(),
            String::new(),
            std::env::consts::OS.to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            String::new(),
            String::new(),
        ),
    };

    // Keep the device heartbeat fresh.
    let _ = state.db.touch_device();

    let resp = StatusResponse {
        device_id,
        device_name,
        hostname,
        platform,
        version,
        location,
        group,
        current_url: display.current_url,
        mode: display.mode,
        online: true,
        scheduler_running: display.scheduler_running,
        uptime_started_at: display.started_at,
        last_successful_url: display.last_successful_url,
        last_successful_at: display.last_successful_at,
    };
    Ok(success(resp))
}

/// `GET /api/v1/device` — full device identity record.
pub async fn get_device(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let device = state
        .db
        .get_device()
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not configured"))?;
    Ok(success(device))
}

/// `PUT /api/v1/device` — update mutable device identity fields.
pub async fn update_device(
    State(state): State<AppState>,
    Json(req): Json<UpdateDevice>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let device = state
        .db
        .update_device(&req)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("device not configured"))?;
    Ok(success(device))
}

/// `GET /api/v1/commands` — recent command audit log.
pub async fn list_commands(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let commands = state.db.list_commands(100).map_err(ApiError::internal)?;
    Ok(success(serde_json::json!({
        "commands": commands,
        "at": now(),
    })))
}

/// `POST /api/v1/restart` — restart the application process.
pub async fn restart(State(_state): State<AppState>) -> ApiResult<impl axum::response::IntoResponse> {
    // Restart by exiting; the OS/service manager (or Tauri) relaunches.
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::process::exit(0);
    });
    Ok(success(serde_json::json!({ "action": "restart" })))
}

/// `POST /api/v1/shutdown` — stop the application process.
pub async fn shutdown(State(_state): State<AppState>) -> ApiResult<impl axum::response::IntoResponse> {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::process::exit(0);
    });
    Ok(success(serde_json::json!({ "action": "shutdown" })))
}

/// `POST /api/v1/reload` — reload the current page and refresh status.
pub async fn reload(State(state): State<AppState>) -> ApiResult<impl axum::response::IntoResponse> {
    state.controller.refresh().map_err(ApiError::internal)?;
    state.controller.emit_status();
    Ok(success(serde_json::json!({ "action": "reload" })))
}