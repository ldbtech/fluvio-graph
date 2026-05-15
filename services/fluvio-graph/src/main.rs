//! fluvio-graph — graph engine service
//!
//! Responsibilities:
//!   - Connect to SurrealDB (twin graph)
//!   - Load DomainGraph into memory on boot
//!   - Serve GraphQL subgraph for graph operations
//!   - No auth, no Postgres, no product logic

use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (dev only — Docker injects env vars directly)
    let _ = dotenvy::dotenv();

    // ORT is very noisy at INFO — silence it regardless of RUST_LOG
    let filter = match std::env::var("RUST_LOG") {
        Ok(s) if !s.trim().is_empty() => {
            let combined = format!("{},ort::logging=warn", s.trim());
            EnvFilter::from_str(&combined)
                .unwrap_or_else(|_| EnvFilter::new("info,ort::logging=warn"))
        }
        _ => EnvFilter::new("info,ort::logging=warn"),
    };
    fmt().with_env_filter(filter).init();

    tracing::info!("fluvio-graph starting...");

    fluvio_graph::server::serve().await
}
