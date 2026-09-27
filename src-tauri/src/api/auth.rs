use crate::api::{ApiError, AppState};
use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use ipnet::IpNet;
use std::net::{IpAddr, SocketAddr};

/// Check whether the client IP is permitted by the configured allowlist.
/// An empty allowlist means "allow all".
fn ip_allowed(ip: IpAddr, allowlist: &str) -> bool {
    let trimmed = allowlist.trim();
    if trimmed.is_empty() {
        return true;
    }
    let entries: Vec<&str> = trimmed
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if entries.is_empty() {
        return true;
    }
    for entry in entries {
        // Support both single IPs and CIDR ranges.
        if let Ok(net) = entry.parse::<IpNet>() {
            if net.contains(&ip) {
                return true;
            }
        } else if let Ok(single) = entry.parse::<IpAddr>() {
            if single == ip {
                return true;
            }
        }
    }
    false
}

/// Loosen an IPv6-mapped IPv4 loopback (`::ffff:127.0.0.1`) to `127.0.0.1`.
fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                IpAddr::V4(v4)
            } else {
                IpAddr::V6(v6)
            }
        }
        other => other,
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Extract the presented credential from the request headers.
fn presented_key(req: &Request<Body>) -> Option<String> {
    // X-API-Key: <key>
    if let Some(v) = req.headers().get("x-api-key") {
        if let Ok(s) = v.to_str() {
            return Some(s.trim().to_string());
        }
    }
    // Authorization: Bearer <key>
    if let Some(v) = req.headers().get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = v.to_str() {
            let s = s.trim();
            if let Some(rest) = s
                .strip_prefix("Bearer ")
                .or_else(|| s.strip_prefix("bearer "))
            {
                return Some(rest.trim().to_string());
            }
        }
    }
    None
}

/// Middleware enforcing IP allowlisting and API key / bearer authentication.
///
/// The health-check path and preflight OPTIONS requests are always allowed so
/// external systems can probe availability before authenticating.
pub async fn auth_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Allow CORS preflight through without auth.
    if req.method() == axum::http::Method::OPTIONS {
        return next.run(req).await;
    }

    let path = req.uri().path().to_string();
    if path == "/health" {
        return next.run(req).await;
    }

    let db = &state.db;

    // 1. IP allowlisting.
    let allowlist = db.get_setting_or("allowed_ips", "");
    let client_ip = normalize_ip(addr.ip());
    if !ip_allowed(client_ip, &allowlist) {
        return ApiError {
            status: StatusCode::FORBIDDEN,
            message: format!("IP {} is not allowed", client_ip),
        }
        .into_response();
    }

    // Local loopback is always trusted (the admin UI on the same machine).
    let is_loopback = client_ip.is_loopback();

    // 2. Authentication.
    let auth_mode = db.get_setting_or("auth_mode", "apikey");
    let expected = db.get_setting_or("api_key", "");
    let auth_disabled = auth_mode.eq_ignore_ascii_case("none") || expected.trim().is_empty();

    if !is_loopback && !auth_disabled {
        match presented_key(&req) {
            Some(key) if constant_time_eq(&key, expected.trim()) => {}
            _ => {
                return ApiError::unauthorized(
                    "Missing or invalid credentials. Provide X-API-Key or Authorization: Bearer <key>.",
                )
                .into_response();
            }
        }

        // Remote control must be explicitly allowed for non-loopback clients.
        let allow_remote = db.get_setting_or("allow_remote", "true");
        if allow_remote.eq_ignore_ascii_case("false") {
            return ApiError::forbidden("Remote control is disabled").into_response();
        }
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_allowlist_allows_all() {
        assert!(ip_allowed("10.0.0.5".parse().unwrap(), ""));
    }

    #[test]
    fn cidr_allowlist() {
        let ip: IpAddr = "192.168.1.50".parse().unwrap();
        assert!(ip_allowed(ip, "192.168.1.0/24"));
        assert!(!ip_allowed(ip, "10.10.0.0/16"));
        assert!(ip_allowed(ip, "192.168.1.0/24, 10.10.0.0/16"));
    }

    #[test]
    fn single_ip_allowlist() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(ip_allowed(ip, "127.0.0.1"));
    }

    #[test]
    fn constant_time_eq_works() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "abcd"));
    }
}