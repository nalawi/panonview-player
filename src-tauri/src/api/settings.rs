use crate::api::{success, ApiError, ApiResult, AppState};
use axum::extract::State;
use serde::Deserialize;
use std::collections::HashMap;

/// Settings that must never be returned in cleartext.
const SECRET_KEYS: [&str; 1] = ["api_key"];

fn mask(key: &str, value: &str) -> String {
    if SECRET_KEYS.contains(&key) {
        if value.is_empty() {
            String::new()
        } else {
            "*".repeat(12)
        }
    } else {
        value.to_string()
    }
}

/// `GET /api/v1/settings` — return all settings (secrets masked).
pub async fn get_settings(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let all = state.db.all_settings().map_err(ApiError::internal)?;
    let mut map = serde_json::Map::new();
    for (k, v) in all {
        map.insert(k.clone(), serde_json::Value::String(mask(&k, &v)));
    }
    Ok(success(serde_json::Value::Object(map)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub settings: HashMap<String, String>,
}

/// `PUT /api/v1/settings` — update one or more settings.
///
/// A masked secret (all asterisks) is ignored so the UI can round-trip the
/// masked value without overwriting the real secret.
pub async fn update_settings(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<UpdateSettingsRequest>,
) -> ApiResult<impl axum::response::IntoResponse> {
    for (k, v) in &req.settings {
        if SECRET_KEYS.contains(&k.as_str()) && v.chars().all(|c| c == '*') {
            continue;
        }
        state.db.set_setting(k, v).map_err(ApiError::internal)?;
    }

    let all = state.db.all_settings().map_err(ApiError::internal)?;
    let mut map = serde_json::Map::new();
    for (k, v) in all {
        map.insert(k.clone(), serde_json::Value::String(mask(&k, &v)));
    }
    Ok(success(serde_json::Value::Object(map)))
}

/// `POST /api/v1/settings/regenerate-key` — issue a new API key.
///
/// The new key is returned in cleartext exactly once; subsequent calls to
/// `GET /api/v1/settings` only ever see the masked value.
pub async fn regenerate_api_key(
    State(state): State<AppState>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let key = uuid::Uuid::new_v4().simple().to_string();
    state
        .db
        .set_setting("api_key", &key)
        .map_err(ApiError::internal)?;
    Ok(success(serde_json::json!({ "api_key": key })))
}
