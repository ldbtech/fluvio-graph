//! database/pool.rs
//! 
//! PostgreSQL connection pool setup . 
//! Call setup_database() once at the server startup.
//! Returns a PgPool that gets stored in AppState.
//! 
use sqlx::PgPool;

pub async fn setup_database() -> anyhow::Result<PgPool> {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/fluvio_twin".to_string());

    let pool = PgPool::connect(&db_url).await
                .map_err(|e| anyhow::anyhow!("Failed to connect to database: {} : {}", db_url, e))?;

    sqlx::migrate!("./migrations").run(&pool).await
                .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;

    tracing::info!("[DB] Connected and migrations run successfully");
    
    Ok(pool)
}