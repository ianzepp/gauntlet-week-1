//! Router assembly.

pub mod auth;
pub mod users;
pub mod ws;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::state::AppState;

/// Shared API routes used by both the React and Leptos servers.
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
        .route("/api/auth/ws-ticket", post(auth::ws_ticket))
        .route("/api/users/{id}/profile", get(users::user_profile))
        .route("/api/ws", get(ws::handle_ws))
        .route("/healthz", get(healthz))
        .layer(cors)
        .with_state(state)
}

/// React frontend: API routes + static file serving with SPA fallback.
pub fn react_app(state: AppState) -> Router {
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "../client/dist".to_string());
    let index_path = format!("{static_dir}/index.html");

    let serve = ServeDir::new(&static_dir).fallback(ServeFile::new(&index_path));

    api_routes(state).fallback_service(serve)
}

/// Leptos SSR frontend: API routes + Leptos SSR routes.
pub fn leptos_app(state: AppState) -> Router {
    let conf = get_configuration(None).expect("leptos configuration");
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(client_rust::app::App);

    let leptos_router = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || client_rust::app::shell(opts.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(client_rust::app::shell))
        .with_state(leptos_options);

    api_routes(state).merge(leptos_router)
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
