//! Embedding fluvio-graph in your own process — no server, no gateway.
//!
//! This is the shape a commercial or private backend uses: link the facade
//! crate, inject config, and query the graph in-process. Everything here goes
//! through `fluvio_graph`'s public surface; nothing reaches into the internal
//! `*-core` crates.
//!
//! Run it against a SurrealDB instance:
//!
//! ```text
//! surreal start --user root --pass root ws://127.0.0.1:8000
//! cargo run -p embedded-consumer
//! ```
//!
//! Point it elsewhere with SURREAL_URL — note that reading the environment is
//! *this binary's* job. The library only ever accepts an explicit config.

use std::sync::Arc;

use fluvio_graph::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Connect. Config is injected, never read from the environment by the
    //    library — that is the whole point of SurrealConfig.
    let cfg = SurrealConfig {
        url: std::env::var("SURREAL_URL").unwrap_or_else(|_| "ws://127.0.0.1:8000".into()),
        ..SurrealConfig::default()
    };
    println!("connecting to {} …", cfg.url);

    let store = Arc::new(SurrealStorage::connect(&cfg).await?);
    store.init_schema().await?;
    println!("storage ready");

    // 2. Load the embedding model. This is the expensive step; hold onto it.
    let mut embedder = EmbeddingContext::new()
        .map_err(|e| anyhow::anyhow!("embedding model failed to load: {e}"))?;
    println!("embedding model ready");

    // 3. Ask a question and get back a grounded subgraph.
    let owner = Uuid::nil(); // your tenant's owner id
    let question = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "what do we know?".to_string());

    let ctx = QueryContext::from_text(
        owner,
        &question,
        &QueryConfig::default(),
        &store,
        &mut embedder,
        None, // workspace filter
    )
    .await?;

    println!(
        "\nquestion: {question}\nretrieved {} nodes (from {} candidates)",
        ctx.node_count, ctx.fetched_count
    );

    if ctx.node_count == 0 {
        println!(
            "\nThe graph is empty for this owner. Ingest something first — see the \
             ingestion pipeline behind the `ingestion` feature, or run the full stack \
             with `docker compose up` and ingest through the gateway."
        );
    }

    Ok(())
}
