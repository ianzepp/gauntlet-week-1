//! Database initialization and migration runner.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Initialize the PostgreSQL connection pool and run migrations.
///
/// # Errors
///
/// Returns an error if the connection or migrations fail.
pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::migrate!("src/db/migrations").run(&pool).await?;

    Ok(pool)
}
