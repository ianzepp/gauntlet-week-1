mod db;
mod frame;
mod llm;
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
    let state = state::AppState::new(pool);

    // Spawn background persistence task.
    let _persistence = services::persistence::spawn_persistence_task(state.clone());

    let app = routes::app(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");

    tracing::info!(%port, "collaboard listening");
    axum::serve(listener, app).await.expect("server failed");
}
