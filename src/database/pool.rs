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

    let skip_migrate = std::env::var("SKIP_SQLX_MIGRATE").ok().as_deref() == Some("1");
    if skip_migrate {
        tracing::warn!(
            "[DB] SKIP_SQLX_MIGRATE=1 — skipping embedded sqlx migrations (schema must already match ./migrations)"
        );
    } else {
        sqlx::migrate!("./migrations").run(&pool).await
                    .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    }

    if skip_migrate {
        tracing::info!("[DB] Connected (migrations skipped)");
    } else {
        tracing::info!("[DB] Connected and migrations applied");
    }
    
    Ok(pool)
}