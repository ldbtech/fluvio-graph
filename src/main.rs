use anyhow::Context;

/// Runs the kg-engine HTTP API (PDF upload, Gmail sync, graph endpoints, etc.).
/// Configure `ANTHROPIC_API_KEY` in the environment or `.env` in the working directory.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let api_key = std::env::var("ANTHROPIC_API_KEY").context(
        "ANTHROPIC_API_KEY not set — add it to `.env` (ANTHROPIC_API_KEY=...) or export ANTHROPIC_API_KEY=sk-ant-...",
    )?;
    kg_engine::server::serve(api_key).await
}
