//! Router assembly.

pub mod auth;
pub mod users;
pub mod ws;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Serve the built frontend from ../client/dist (fallback to index.html for SPA routing).
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "../client/dist".into());
    let spa = ServeDir::new(&static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html")));

    Router::new()
        // Public OAuth routes.
        .route("/auth/github", get(auth::github_redirect))
        .route("/auth/github/callback", get(auth::github_callback))
        // Authenticated API routes.
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/ws-ticket", post(auth::ws_ticket))
        // User profile.
        .route("/api/users/{id}/profile", get(users::user_profile))
        // WebSocket (ticket-gated).
        .route("/api/ws", get(ws::handle_ws))
        // Health check.
        .route("/healthz", get(healthz))
        .layer(cors)
        .with_state(state)
        // Fallback: serve static frontend files (must be last).
        .fallback_service(spa)
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
