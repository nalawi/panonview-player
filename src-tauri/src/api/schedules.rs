use crate::api::{success, ApiError, ApiResult, AppState};
use crate::models::{NewSchedule, UpdateSchedule};
use axum::extract::{Path, State};
use axum::Json;

/// `GET /api/v1/schedules` — list all schedules.
pub async fn list_schedules(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let schedules = state.db.list_schedules().map_err(ApiError::internal)?;
    Ok(success(serde_json::json!({ "schedules": schedules })))
}

/// `GET /api/v1/schedules/:id` — fetch a single schedule.
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let schedule = state
        .db
        .get_schedule(id)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("schedule {} not found", id)))?;
    Ok(success(schedule))
}

/// `POST /api/v1/schedules` — create a schedule.
pub async fn create_schedule(
    State(state): State<AppState>,
    Json(req): Json<NewSchedule>,
) -> ApiResult<impl axum::response::IntoResponse> {
    // The referenced page must exist.
    if state
        .db
        .get_page(req.page_id)
        .map_err(ApiError::internal)?
        .is_none()
    {
        return Err(ApiError::bad_request(format!(
            "page {} does not exist",
            req.page_id
        )));
    }
    let schedule = state.db.create_schedule(&req).map_err(ApiError::internal)?;
    state.controller.tick();
    Ok(success(schedule))
}

/// `PUT /api/v1/schedules/:id` — update a schedule.
pub async fn update_schedule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateSchedule>,
) -> ApiResult<impl axum::response::IntoResponse> {
    if let Some(page_id) = req.page_id {
        if state
            .db
            .get_page(page_id)
            .map_err(ApiError::internal)?
            .is_none()
        {
            return Err(ApiError::bad_request(format!(
                "page {} does not exist",
                page_id
            )));
        }
    }
    let schedule = state
        .db
        .update_schedule(id, &req)
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found(format!("schedule {} not found", id)))?;
    state.controller.tick();
    Ok(success(schedule))
}

/// `DELETE /api/v1/schedules/:id` — delete a schedule.
pub async fn delete_schedule(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let deleted = state.db.delete_schedule(id).map_err(ApiError::internal)?;
    if !deleted {
        return Err(ApiError::not_found(format!("schedule {} not found", id)));
    }
    state.controller.tick();
    Ok(success(serde_json::json!({ "id": id })))
}