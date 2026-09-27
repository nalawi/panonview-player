use crate::api::{success, ApiError, ApiResult, AppState};
use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SetUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct SetUrlResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct OverrideRequest {
    pub url: String,
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(default)]
    pub priority: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ShowPageRequest {
    pub page_id: i64,
}

/// `GET /api/v1/display` — return the current display state.
pub async fn get_display(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let status = state.controller.status().map_err(ApiError::internal)?;
    Ok(success(status))
}

/// `POST /api/v1/display` — set the displayed URL immediately.
pub async fn set_display(
    State(state): State<AppState>,
    Json(req): Json<SetUrlRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    if req.url.trim().is_empty() {
        return Err(ApiError::bad_request("url must not be empty"));
    }
    state
        .controller
        .set_url(req.url.trim())
        .map_err(ApiError::internal)?;
    state.controller.emit_status();
    Ok(success(SetUrlResponse { url: req.url }))
}

/// `POST /api/v1/display/page` — show a stored page by id.
pub async fn show_page(
    State(state): State<AppState>,
    Json(req): Json<ShowPageRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    state
        .controller
        .show_page(req.page_id)
        .map_err(ApiError::not_found)?;
    state.controller.emit_status();
    Ok(success(SetUrlResponse {
        url: state
            .controller
            .status()
            .map_err(ApiError::internal)?
            .current_url
            .unwrap_or_default(),
    }))
}

/// `POST /api/v1/display/refresh` — reload the current page.
pub async fn refresh(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    state.controller.refresh().map_err(ApiError::internal)?;
    Ok(success(serde_json::json!({})))
}

/// `POST /api/v1/display/back` — navigate back.
pub async fn back(State(state): State<AppState>) -> ApiResult<impl axum::response::IntoResponse> {
    let url = state.controller.back().map_err(ApiError::internal)?;
    state.controller.emit_status();
    Ok(success(serde_json::json!({ "url": url })))
}

/// `POST /api/v1/display/forward` — navigate forward.
pub async fn forward(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let url = state.controller.forward().map_err(ApiError::internal)?;
    state.controller.emit_status();
    Ok(success(serde_json::json!({ "url": url })))
}

/// `POST /api/v1/display/reset` — clear the display (default standby screen).
pub async fn reset(State(state): State<AppState>) -> ApiResult<impl axum::response::IntoResponse> {
    state.controller.reset().map_err(ApiError::internal)?;
    Ok(success(serde_json::json!({})))
}

/// `POST /api/v1/display/override` — temporarily supersede the scheduler.
pub async fn override_display(
    State(state): State<AppState>,
    Json(req): Json<OverrideRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    if req.url.trim().is_empty() {
        return Err(ApiError::bad_request("url must not be empty"));
    }
    let duration = req.duration.unwrap_or(300).max(1);
    let priority = req.priority.unwrap_or(100);
    state
        .controller
        .override_page(req.url.trim(), duration, priority)
        .map_err(ApiError::internal)?;
    state.controller.emit_status();
    Ok(success(serde_json::json!({
        "url": req.url,
        "duration": duration,
        "priority": priority,
        "mode": "override"
    })))
}