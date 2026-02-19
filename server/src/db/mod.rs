//! Database initialization and migration runner.
//!
//! SYSTEM CONTEXT
//! ==============
//! Startup uses this module to create the shared SQLx pool and enforce schema
//! migrations before accepting websocket/API traffic.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

const DEFAULT_DB_MAX_CONNECTIONS: u32 = 5;

fn db_max_connections() -> u32 {
    std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DB_MAX_CONNECTIONS)
}

/// Initialize the `PostgreSQL` connection pool and run migrations.
///
/// # Errors
///
/// Returns an error if the connection or migrations fail.
pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(db_max_connections())
        .connect(database_url)
        .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;

    Ok(pool)
}
