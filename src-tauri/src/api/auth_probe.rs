use crate::api::{success, ApiResult, AppState};
use axum::extract::{ConnectInfo, State};
use std::net::SocketAddr;

/// `GET /api/v1/auth` — unauthenticated bootstrap probe used by the browser UI.
///
/// Consumers need to know two things before they can talk to the API:
///   1. Whether this client is trusted (loopback) and can therefore be given
///      the API key automatically, so the local machine does not have to type
///      it in.
///   2. Whether authentication is required at all.
///
/// The key is returned **only** to loopback clients, matching the trust model
/// already enforced by the auth middleware. Remote clients get just a hint
/// that a key is required.
pub async fn auth_probe(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> ApiResult<impl axum::response::IntoResponse> {
    let ip = addr.ip();
    let is_loopback = ip.is_loopback()
        || matches!(
            ip,
            std::net::IpAddr::V6(v6) if v6.to_ipv4_mapped().map(|v| v.is_loopback()).unwrap_or(false)
        );

    let auth_mode = state.db.get_setting_or("auth_mode", "apikey");
    let auth_required = !auth_mode.eq_ignore_ascii_case("none");

    if is_loopback {
        Ok(success(serde_json::json!({
            "loopback": true,
            "auth_required": auth_required,
            "api_key": state.db.get_setting_or("api_key", ""),
        })))
    } else {
        Ok(success(serde_json::json!({
            "loopback": false,
            "auth_required": auth_required,
        })))
    }
}