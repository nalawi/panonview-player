pub mod auth;
pub mod auth_probe;
pub mod display;
pub mod pages;
pub mod routes;
pub mod schedules;
pub mod settings;
pub mod system;

use crate::database::Database;
use crate::display::DisplayController;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

/// Shared state passed to every HTTP handler.
#[derive(Clone)]
pub struct AppState {
    pub controller: DisplayController,
    pub db: Database,
    /// Directory containing the built admin web UI, served at `/ui`.
    pub ui_dir: Option<PathBuf>,
}

/// Simple API error type that renders a JSON body with an HTTP status.
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: msg.into(),
        }
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "success": false,
            "error": self.message,
        }));
        (self.status, body).into_response()
    }
}

/// Convenience alias for handler results.
pub type ApiResult<T> = Result<T, ApiError>;

/// A success envelope matching the documented API responses.
#[derive(Serialize)]
pub struct Success<T: Serialize> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

pub fn success<T: Serialize>(data: T) -> Json<Success<T>> {
    Json(Success {
        success: true,
        data,
    })
}

/// Start the embedded HTTP server. Returns once the listener is bound; the
/// server keeps running on the shared tokio runtime.
pub async fn start_server(state: AppState, bind: String, port: u16) -> Result<(), String> {
    let addr = format!("{}:{}", bind, port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("failed to bind {}: {}", addr, e))?;

    let app = routes::build_router(state);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            eprintln!("[http] server error: {}", e);
        }
    });

    Ok(())
}