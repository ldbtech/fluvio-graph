//! fluvio-twin — digital twin product service
//!
//! The LLM chat layer over the knowledge graph.
//! Calls fluvio-graph for semantic retrieval, assembles context, streams Claude responses.

use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let filter = match std::env::var("RUST_LOG") {
        Ok(s) if !s.trim().is_empty() => {
            let combined = format!("{},ort::logging=warn,tokenizers=warn", s.trim());
            EnvFilter::from_str(&combined)
                .unwrap_or_else(|_| EnvFilter::new("info,ort::logging=warn"))
        }
        _ => EnvFilter::new("info,ort::logging=warn"),
    };
    fmt().with_env_filter(filter).init();

    tracing::info!("fluvio-twin starting...");
    fluvio_twin::server::serve().await
}