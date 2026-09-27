use crate::api::{auth, auth_probe, display, pages, schedules, settings, system, AppState};
use axum::response::Redirect;
use axum::routing::{get, post};
use axum::{middleware, Router};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

/// Build the full API router.
///
/// - `/api/v1/*` requires authentication + IP allowlisting.
/// - `/health` is an unauthenticated liveness probe.
/// - `/ui` serves the built admin web console (static assets, no auth).
/// - `/` redirects to `/ui/`.
pub fn build_router(state: AppState) -> Router {
    // Everything that requires authentication.
    let protected = Router::new()
        // Display
        .route(
            "/api/v1/display",
            get(display::get_display).post(display::set_display),
        )
        .route("/api/v1/display/page", post(display::show_page))
        .route("/api/v1/display/refresh", post(display::refresh))
        .route("/api/v1/display/back", post(display::back))
        .route("/api/v1/display/forward", post(display::forward))
        .route("/api/v1/display/reset", post(display::reset))
        .route("/api/v1/display/override", post(display::override_display))
        // Pages
        .route(
            "/api/v1/pages",
            get(pages::list_pages).post(pages::create_page),
        )
        .route(
            "/api/v1/pages/:id",
            get(pages::get_page)
                .put(pages::update_page)
                .delete(pages::delete_page),
        )
        // Schedules
        .route(
            "/api/v1/schedules",
            get(schedules::list_schedules).post(schedules::create_schedule),
        )
        .route(
            "/api/v1/schedules/:id",
            get(schedules::get_schedule)
                .put(schedules::update_schedule)
                .delete(schedules::delete_schedule),
        )
        // Settings
        .route(
            "/api/v1/settings",
            get(settings::get_settings).put(settings::update_settings),
        )
        .route(
            "/api/v1/settings/regenerate-key",
            post(settings::regenerate_api_key),
        )
        // System
        .route("/api/v1/status", get(system::get_status))
        .route(
            "/api/v1/device",
            get(system::get_device).put(system::update_device),
        )
        .route("/api/v1/commands", get(system::list_commands))
        .route("/api/v1/restart", post(system::restart))
        .route("/api/v1/shutdown", post(system::shutdown))
        .route("/api/v1/reload", post(system::reload))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .with_state(state.clone());

    // Public routes: health + auth probe + the admin web console + redirect.
    let mut public = Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth", get(auth_probe::auth_probe))
        .route("/", get(root_redirect));

    // Serve the built UI assets when available (best-effort; the API works
    // even if the UI bundle is absent).
    if let Some(dir) = state.ui_dir.clone() {
        if dir.exists() {
            public = public.nest_service("/ui", ServeDir::new(dir));
        }
    }
    let public = public.with_state(state);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new().merge(protected).merge(public).layer(cors)
}

/// Liveness probe — unauthenticated.
async fn health() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "success": true,
        "status": "ok",
        "service": "tauri-web-player",
    }))
}

/// Redirect the site root to the admin web console.
async fn root_redirect() -> Redirect {
    Redirect::temporary("/ui/")
}