#![allow(dead_code)]

mod db;
mod frame;
mod llm;
mod rate_limit;
mod routes;
mod services;
mod state;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .expect("invalid PORT");

    let pool = db::init_pool(&database_url)
        .await
        .expect("database init failed");

    // Initialize LLM client (non-fatal: AI features disabled if config missing).
    let llm: Option<std::sync::Arc<dyn llm::LlmChat>> = match llm::LlmClient::from_env() {
        Ok(client) => {
            tracing::info!(model = client.model(), "LLM client initialized");
            Some(std::sync::Arc::new(client))
        }
        Err(e) => {
            tracing::warn!(error = %e, "LLM client not configured — AI features disabled");
            None
        }
    };

    // Initialize GitHub OAuth config (non-fatal: auth disabled if env vars missing).
    let github = services::auth::GitHubConfig::from_env();
    if github.is_some() {
        tracing::info!("GitHub OAuth configured");
    } else {
        tracing::warn!(
            "GitHub OAuth not configured — GITHUB_CLIENT_ID / GITHUB_CLIENT_SECRET / GITHUB_REDIRECT_URI missing"
        );
    }

    let state = state::AppState::new(pool, llm, github);

    // Spawn background persistence task.
    let _persistence = services::persistence::spawn_persistence_task(state.clone());

    let app = routes::app(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");

    tracing::info!(%port, "collaboard listening");
    axum::serve(listener, app).await.expect("server failed");
}
