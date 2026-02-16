//! Router assembly.

pub mod auth;
pub mod ws;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Public OAuth routes.
        .route("/auth/github", get(auth::github_redirect))
        .route("/auth/github/callback", get(auth::github_callback))
        // Authenticated API routes.
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/ws-ticket", post(auth::ws_ticket))
        // WebSocket (ticket-gated).
        .route("/api/ws", get(ws::handle_ws))
        // Health check.
        .route("/healthz", get(healthz))
        .layer(cors)
        .with_state(state)
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
