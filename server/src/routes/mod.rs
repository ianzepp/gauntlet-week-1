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

use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Leptos SSR configuration.
    let conf = get_configuration(None).expect("leptos configuration");
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(client_rust::app::App);

    // API routes use AppState.
    let api = Router::new()
        .route("/auth/github", get(auth::github_redirect))
        .route("/auth/github/callback", get(auth::github_callback))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/ws-ticket", post(auth::ws_ticket))
        .route("/api/users/{id}/profile", get(users::user_profile))
        .route("/api/ws", get(ws::handle_ws))
        .route("/healthz", get(healthz))
        .layer(cors)
        .with_state(state);

    // Leptos SSR routes use LeptosOptions.
    let leptos_app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || client_rust::app::shell(opts.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(client_rust::app::shell))
        .with_state(leptos_options);

    // Merge: API routes take priority, Leptos handles everything else.
    api.merge(leptos_app)
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}
