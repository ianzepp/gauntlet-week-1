//! Router assembly.

pub mod ws;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/ws", get(ws::handle_ws))
        .layer(cors)
        .with_state(state)
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
