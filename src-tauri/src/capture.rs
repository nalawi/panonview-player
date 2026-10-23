//! Screen capture helpers shared by the Tauri IPC command and the HTTP API.
//!
//! Captures only the player window (title "PanonView Player") via `xcap`,
//! falling back to the primary monitor if the window cannot be found.
//! On macOS this requires Screen Recording permission (System Settings →
//! Privacy & Security).

use image::{codecs::png::PngEncoder, ImageEncoder};

/// The player window title, as configured in tauri.conf.json.
const PLAYER_WINDOW_TITLE: &str = "PanonView Player";

/// Capture the player window (or primary monitor as fallback) as PNG bytes.
pub fn capture_png() -> Result<Vec<u8>, String> {
    // Prefer the player window itself so the snapshot reflects exactly the
    // page the player is currently rendering.
    let img = match xcap::Window::all()
        .map_err(|e| format!("failed to enumerate windows: {e}"))?
        .into_iter()
        .find(|w| w.title().map(|t| t.trim() == PLAYER_WINDOW_TITLE).unwrap_or(false))
    {
        Some(win) => win.capture_image(),
        None => {
            // Fallback: primary monitor.
            xcap::Monitor::all()
                .map_err(|e| format!("failed to enumerate monitors: {e}"))?
                .into_iter()
                .next()
                .ok_or_else(|| "no monitor found".to_string())?
            .capture_image()
        }
    }
    .map_err(|e| format!("capture failed: {e}"))?;

    let (w, h) = img.dimensions();
    let mut png_buf = Vec::new();
    PngEncoder::new(&mut png_buf)
        .write_image(
            img.as_raw(),
            w,
            h,
            image::ColorType::Rgba8.into(),
        )
        .map_err(|e| format!("png encode failed: {e}"))?;

    Ok(png_buf)
}

/// Capture and return a base64-encoded PNG data URL (for Tauri IPC).
pub fn capture_data_url() -> Result<String, String> {
    use base64::Engine;
    let png = capture_png()?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ))
}