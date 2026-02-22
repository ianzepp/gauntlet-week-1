//! Router assembly.
//!
//! SYSTEM CONTEXT
//! ==============
//! This module binds HTTP + websocket endpoints and stitches API routes with
//! Leptos SSR rendering under a single Axum router. The public portfolio site
//! is served as static files at `/`, while the Leptos app lives under `/app`.

pub mod auth;
pub mod boards;
pub mod users;
pub mod ws;

use std::path::PathBuf;

use axum::extract::Path;
use axum::Router;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::{get, patch, post};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::state::AppState;

/// Shared API routes used by the SSR app and websocket clients.
fn api_routes(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/login", get(redirect_login_to_app))
        .route("/board", get(redirect_board_root_to_app_board))
        .route("/board/{id}", get(redirect_board_to_app))
        .route("/auth/github", get(auth::github_redirect))
        .route("/auth/github/callback", get(auth::github_callback))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/email/request-code", post(auth::request_email_code))
        .route("/api/auth/email/verify-code", post(auth::verify_email_code))
        .route("/api/auth/session-token", get(auth::session_token))
        .route("/api/auth/ws-ticket", post(auth::ws_ticket))
        .route("/api/dev/ws-ticket", post(auth::dev_ws_ticket))
        .route(
            "/api/boards/{id}/members",
            get(boards::list_members).post(boards::upsert_member),
        )
        .route("/api/board", get(boards::list_boards_rest).post(boards::create_board_rest))
        .route(
            "/api/board/{id}",
            get(boards::get_board)
                .patch(boards::update_board_rest)
                .delete(boards::delete_board_rest),
        )
        .route(
            "/api/board/{id}/objects",
            get(boards::list_objects).post(boards::create_object_rest),
        )
        .route(
            "/api/board/{id}/objects/{object_id}",
            get(boards::get_object)
                .patch(boards::patch_object)
                .delete(boards::delete_object_rest),
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

async fn redirect_login_to_app() -> Redirect {
    Redirect::temporary("/app/login")
}

async fn redirect_board_to_app(Path(id): Path<String>) -> Redirect {
    Redirect::temporary(&format!("/app/board/{id}"))
}

async fn redirect_board_root_to_app_board() -> Redirect {
    Redirect::temporary("/app/board")
}

/// Resolve the path to the portfolio website directory.
fn website_dir() -> PathBuf {
    std::env::var("WEBSITE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../website"))
}

/// Leptos SSR frontend: API routes + Leptos SSR at `/app` + portfolio at `/`.
///
/// # Errors
///
/// Returns an error if the Leptos configuration cannot be loaded (missing or
/// malformed `Cargo.toml` `[package.metadata.leptos]` section).
pub fn leptos_app(state: AppState) -> Result<Router, String> {
    let conf = get_configuration(None).map_err(|e| format!("leptos configuration: {e}"))?;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(client::app::App);

    // Leptos SSR routes (now under /app prefix via client-side route definitions).
    let leptos_router = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || client::app::shell(opts.clone())
        })
        .with_state(leptos_options.clone());

    // Serve Leptos static assets (WASM, CSS, JS) from the site root /pkg directory.
    let site_root_path = PathBuf::from(leptos_options.site_root.as_ref());

    // Portfolio website served as static files at `/`.
    let website_path = website_dir();
    let website_service = ServeDir::new(&website_path).append_index_html_on_directories(true);

    Ok(api_routes(state)
        .merge(leptos_router)
        .nest_service("/pkg", ServeDir::new(site_root_path.join("pkg")))
        .fallback_service(website_service))
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
