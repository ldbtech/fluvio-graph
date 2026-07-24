//! PostgreSQL connection pool setup and migrations runner.
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5)) // timeout for acquiring a connection from the pool
        .connect(database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create database pool: {}", e))?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
              .run(pool)
              .await
              .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    
    tracing::info!("Migrations applied successfully");
    Ok(())
}
