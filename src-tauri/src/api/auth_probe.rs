use crate::api::{success, ApiResult, AppState};
use axum::extract::{ConnectInfo, State};
use std::net::SocketAddr;

/// `GET /api/v1/auth` — unauthenticated bootstrap probe used by the browser UI.
///
/// It tells the sign-in screen only **whether** credentials are required
/// (`auth_mode`); it never reveals the API key itself. Handing the real key to
/// loopback browsers would defeat the login gate, since the web console is
/// served by the player itself and every browser on the host machine connects
/// from a loopback address. Users read the key from the desktop app
/// (Settings → API Key) or from the local database.
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
    let auth_required = !auth_mode.eq_ignore_ascii_case("none")
        && !state
            .db
            .get_setting_or("api_key", "")
            .trim()
            .is_empty();

    Ok(success(serde_json::json!({
        "loopback": is_loopback,
        "auth_required": auth_required,
    })))
}