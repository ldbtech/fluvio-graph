//! fluvio-database — Postgres data service
//!
//! Pure CRUD over PostgreSQL. No business logic.
//! Every other service that needs Postgres data calls this service.

use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();

    tracing::info!("fluvio-database starting...");
    fluvio_database::server::serve().await
}