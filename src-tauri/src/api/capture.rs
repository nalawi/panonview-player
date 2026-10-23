use crate::capture::capture_png;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// `GET /api/v1/display/screenshot` — PNG snapshot of the current screen.
pub async fn screenshot() -> Result<Response, String> {
    // Run the blocking capture off the async runtime.
    let result =
        tokio::task::spawn_blocking(capture_png)
            .await
            .map_err(|e| format!("capture task failed: {e}"))?;

    match result {
        Ok(bytes) => Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "image/png")],
            bytes,
        )
            .into_response()),
        Err(e) => Err(e),
    }
}