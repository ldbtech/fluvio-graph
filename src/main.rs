use anyhow::Context;
use std::str::FromStr;
use tracing_subscriber::{fmt, EnvFilter};

/// Runs the kg-engine HTTP API (PDF upload, Gmail sync, graph endpoints, etc.).
/// Configure `ANTHROPIC_API_KEY` in the environment or `.env` in the working directory.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    // ONNX Runtime (`ort`) graph-optimizer chatter is very noisy at INFO.
    // If `RUST_LOG` is set (e.g. in `.env`), still append `ort::logging=warn` so ORT stays quiet.
    let filter = match std::env::var("RUST_LOG") {
        Ok(s) if !s.trim().is_empty() => {
            let combined = format!("{},ort::logging=warn", s.trim());
            EnvFilter::from_str(&combined).unwrap_or_else(|_| EnvFilter::new("info,ort::logging=warn"))
        }
        _ => EnvFilter::new("info,ort::logging=warn"),
    };
    fmt().with_env_filter(filter).init();
    let api_key = std::env::var("ANTHROPIC_API_KEY").context(
        "ANTHROPIC_API_KEY not set — add it to `.env` (ANTHROPIC_API_KEY=...) or export ANTHROPIC_API_KEY=sk-ant-...",
    )?;
    kg_engine::server::serve(api_key).await
}
