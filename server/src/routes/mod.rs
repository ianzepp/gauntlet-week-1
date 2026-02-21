//! Router assembly.
//!
//! SYSTEM CONTEXT
//! ==============
//! This module binds HTTP + websocket endpoints and stitches API routes with
//! Leptos SSR rendering under a single Axum router.

pub mod auth;
pub mod boards;
pub mod users;
pub mod ws;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

/// Shared API routes used by the SSR app and websocket clients.
fn api_routes(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/auth/github", get(auth::github_redirect))
        .route("/auth/github/callback", get(auth::github_callback))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/email/request-code", post(auth::request_email_code))
        .route("/api/auth/email/verify-code", post(auth::verify_email_code))
        .route("/api/auth/ws-ticket", post(auth::ws_ticket))
        .route("/api/dev/ws-ticket", post(auth::dev_ws_ticket))
        .route(
            "/api/boards/{id}/members",
            get(boards::list_members).post(boards::upsert_member),
        )
        .route("/api/boards/{id}/import.jsonl", post(boards::import_jsonl))
        .route("/api/boards/{id}/export.jsonl", get(boards::export_jsonl))
        .route(
            "/api/boards/{id}/members/{user_id}",
            patch(boards::update_member).delete(boards::delete_member),
        )
        .route("/api/users/{id}/profile", get(users::user_profile))
        .route("/api/ws", get(ws::handle_ws))
        .route("/healthz", get(healthz))
        .layer(cors)
        .with_state(state)
}

/// Leptos SSR frontend: API routes + Leptos SSR routes.
///
/// # Errors
///
/// Returns an error if the Leptos configuration cannot be loaded (missing or
/// malformed `Cargo.toml` `[package.metadata.leptos]` section).
pub fn leptos_app(state: AppState) -> Result<Router, String> {
    let conf = get_configuration(None).map_err(|e| format!("leptos configuration: {e}"))?;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(client::app::App);

    let leptos_router = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || client::app::shell(opts.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(client::app::shell))
        .with_state(leptos_options);

    Ok(api_routes(state).merge(leptos_router))
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
