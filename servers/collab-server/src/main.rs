//! fluvio-collab — collaborative Knowledge Graph service
//!
//! Pure Orchestration layer for collaborative operations.. No direct data access.
//! All data flows through fluvio-database and fluvio-graph.

use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
 
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
 
    tracing::info!("fluvio-collab starting...");
    collab_server::server::serve().await
}