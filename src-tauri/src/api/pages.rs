use crate::api::{success, ApiError, ApiResult, AppState};
use crate::models::{NewPage, UpdatePage};
use axum::extract::{Path, State};
use axum::Json;

/// `GET /api/v1/pages` — list all pages.
pub async fn list_pages(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let pages = state.db.list_pages().map_err(ApiError::internal)?;
    Ok(success(serde_json::json!({ "pages": pages })))
}

/// `GET /api/v1/pages/:id` — fetch a single page.
pub async fn get_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = state
        .db
        .get_page(id)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("page {} not found", id)))?;
    Ok(success(page))
}

/// `POST /api/v1/pages` — create a page.
pub async fn create_page(
    State(state): State<AppState>,
    Json(req): Json<NewPage>,
) -> ApiResult<impl axum::response::IntoResponse> {
    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request("name must not be empty"));
    }
    if req.url.trim().is_empty() {
        return Err(ApiError::bad_request("url must not be empty"));
    }
    let page = state.db.create_page(&req).map_err(ApiError::internal)?;
    Ok(success(page))
}

/// `PUT /api/v1/pages/:id` — update a page.
pub async fn update_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdatePage>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let page = state
        .db
        .update_page(id, &req)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("page {} not found", id)))?;
    Ok(success(page))
}

/// `DELETE /api/v1/pages/:id` — delete a page.
pub async fn delete_page(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let deleted = state.db.delete_page(id).map_err(ApiError::internal)?;
    if !deleted {
        return Err(ApiError::not_found(format!("page {} not found", id)));
    }
    // If the deleted page was on screen, let the scheduler pick a new target.
    state.controller.tick();
    Ok(success(serde_json::json!({ "id": id })))
}