#![allow(dead_code)]

mod db;
mod frame;
mod llm;
mod rate_limit;
mod routes;
mod services;
mod state;

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn has_flag(flag: &str) -> bool {
    std::env::args().any(|arg| arg == flag)
}

fn env_parse_or<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr + Copy,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

fn env_is_set(key: &str) -> bool {
    std::env::var(key).is_ok()
}

fn log_env_line(key: &str, value: impl std::fmt::Display) {
    tracing::info!("using env: {key}={value}");
}

fn log_startup_env_config(port: u16) {
    let llm_api_key_env = std::env::var("LLM_API_KEY_ENV").ok();
    let llm_api_key_set = llm_api_key_env
        .as_ref()
        .is_some_and(|key_name| std::env::var(key_name).is_ok());

    log_env_line("HOST", env_or_default("HOST", "0.0.0.0"));
    log_env_line("PORT", port);
    log_env_line("STATIC_DIR", env_or_default("STATIC_DIR", "../client/dist"));

    log_env_line("DATABASE_URL_SET", env_is_set("DATABASE_URL"));
    log_env_line("DB_MAX_CONNECTIONS", env_parse_or("DB_MAX_CONNECTIONS", 5_u32));

    log_env_line("LLM_PROVIDER", env_or_default("LLM_PROVIDER", "anthropic"));
    log_env_line("LLM_MODEL", env_or_default("LLM_MODEL", "<default-by-provider>"));
    log_env_line("LLM_API_KEY_ENV", llm_api_key_env.unwrap_or_else(|| "<unset>".to_string()));
    log_env_line("LLM_API_KEY_SET", llm_api_key_set);
    log_env_line("LLM_OPENAI_MODE", env_or_default("LLM_OPENAI_MODE", "responses"));
    log_env_line(
        "LLM_OPENAI_BASE_URL",
        env_or_default("LLM_OPENAI_BASE_URL", "https://api.openai.com/v1"),
    );
    log_env_line("LLM_REQUEST_TIMEOUT_SECS", env_parse_or("LLM_REQUEST_TIMEOUT_SECS", 120_u64));
    log_env_line("LLM_CONNECT_TIMEOUT_SECS", env_parse_or("LLM_CONNECT_TIMEOUT_SECS", 10_u64));

    log_env_line(
        "WS_CLIENT_CHANNEL_CAPACITY",
        env_parse_or("WS_CLIENT_CHANNEL_CAPACITY", 256_usize),
    );
    log_env_line("OBJECT_FLUSH_INTERVAL_MS", env_parse_or("OBJECT_FLUSH_INTERVAL_MS", 100_u64));

    log_env_line("AI_MAX_TOOL_ITERATIONS", env_parse_or("AI_MAX_TOOL_ITERATIONS", 10_usize));
    log_env_line("AI_MAX_TOKENS", env_parse_or("AI_MAX_TOKENS", 4096_u32));
    log_env_line("RATE_LIMIT_PER_CLIENT", env_parse_or("RATE_LIMIT_PER_CLIENT", 10_usize));
    log_env_line(
        "RATE_LIMIT_PER_CLIENT_WINDOW_SECS",
        env_parse_or("RATE_LIMIT_PER_CLIENT_WINDOW_SECS", 60_u64),
    );
    log_env_line("RATE_LIMIT_GLOBAL", env_parse_or("RATE_LIMIT_GLOBAL", 20_usize));
    log_env_line(
        "RATE_LIMIT_GLOBAL_WINDOW_SECS",
        env_parse_or("RATE_LIMIT_GLOBAL_WINDOW_SECS", 60_u64),
    );
    log_env_line("RATE_LIMIT_TOKEN_BUDGET", env_parse_or("RATE_LIMIT_TOKEN_BUDGET", 50_000_u64));
    log_env_line(
        "RATE_LIMIT_TOKEN_WINDOW_SECS",
        env_parse_or("RATE_LIMIT_TOKEN_WINDOW_SECS", 3600_u64),
    );

    log_env_line(
        "FRAME_PERSIST_QUEUE_CAPACITY",
        env_parse_or("FRAME_PERSIST_QUEUE_CAPACITY", 8192_usize),
    );
    log_env_line("FRAME_PERSIST_BATCH_SIZE", env_parse_or("FRAME_PERSIST_BATCH_SIZE", 128_usize));
    log_env_line("FRAME_PERSIST_FLUSH_MS", env_parse_or("FRAME_PERSIST_FLUSH_MS", 5_u64));
    log_env_line("FRAME_PERSIST_RETRIES", env_parse_or("FRAME_PERSIST_RETRIES", 2_usize));
    log_env_line(
        "FRAME_PERSIST_RETRY_BASE_MS",
        env_parse_or("FRAME_PERSIST_RETRY_BASE_MS", 20_u64),
    );

    log_env_line("GITHUB_CLIENT_ID_SET", env_is_set("GITHUB_CLIENT_ID"));
    log_env_line("GITHUB_CLIENT_SECRET_SET", env_is_set("GITHUB_CLIENT_SECRET"));
    log_env_line("GITHUB_REDIRECT_URI", env_or_default("GITHUB_REDIRECT_URI", "<unset>"));
    tracing::info!(
        "using env: NOTES=Secrets omitted; values like DATABASE_URL, API keys, and OAuth client secrets are never logged"
    );
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let migrate_only = has_flag("--migrate-only");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse()
        .expect("invalid PORT");
    log_startup_env_config(port);

    let pool = db::init_pool(&database_url)
        .await
        .expect("database init failed");
    if migrate_only {
        tracing::info!("migrations completed in --migrate-only mode; exiting");
        drop(pool);
        return;
    }

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

    let mut app_state = state::AppState::new(pool, llm, github);
    app_state.frame_persist_tx = Some(services::persistence::spawn_frame_persistence_worker(app_state.pool.clone()));

    // Spawn background persistence task.
    let _persistence = services::persistence::spawn_persistence_task(app_state.clone());

    // Build the combined Axum + Leptos router.
    let app = routes::app(app_state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");

    tracing::info!(%port, "gauntlet-week-1 listening");
    axum::serve(listener, app).await.expect("server failed");
}
