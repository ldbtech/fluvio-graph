//! fluvio-ingestion — data intake pipeline service
//!
//! Receives raw files and text, extracts content, chunks, embeds,
//! and writes nodes to fluvio-graph via GraphQL.

use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let filter = match std::env::var("RUST_LOG") {
        Ok(s) if !s.trim().is_empty() => {
            let combined = format!("{},ort::logging=warn,tokenizers=warn", s.trim());
            EnvFilter::from_str(&combined)
                .unwrap_or_else(|_| EnvFilter::new("info,ort::logging=warn,tokenizers=warn"))
        }
        _ => EnvFilter::new("info,ort::logging=warn,tokenizers=warn"),
    };
    fmt().with_env_filter(filter).init();

    tracing::info!("fluvio-ingestion starting...");

    fluvio_ingestion::server::serve().await
}