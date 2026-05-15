//! Axum server for fluvio-graph.
//!
//! AppState holds:
//!   - SurrealDB connection (the graph lives here, not in RAM)
//!   - Shared embedding model (BGE-small, ~130MB, loaded once)
//!   - GraphRegistry (meta-graph for cross-domain links)
//!
//! There is NO global DomainGraph in AppState.
//! Per-request subgraphs are built inside QueryContext and dropped after each request.

use std::sync::Arc;
use axum::{Router, routing::get, http::Method};
use tower_http::cors::{Any, CorsLayer};
use tokio::sync::RwLock;

use crate::storage::surreal::SurrealStorage;
use crate::registry::GraphRegistry;
use crate::embeddings::EmbeddingContext;

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// SurrealDB — the graph lives here permanently.
    /// QueryContext fetches subgraphs from here per request.
    pub surreal:  Arc<SurrealStorage>,

    /// BGE-small embedding model — shared across all requests.
    /// ~130MB fixed RAM cost. Loaded once on boot.
    /// Wrapped in RwLock because embed() takes &mut self.
    pub embedder: Arc<RwLock<EmbeddingContext>>,

    /// Meta-graph registry for cross-domain links.
    /// Small — only holds ExternalRef nodes.
    pub registry: Arc<RwLock<GraphRegistry>>,
}

// ── serve() ───────────────────────────────────────────────────────────────────

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());

    // ── SurrealDB ─────────────────────────────────────────────────────────────
    tracing::info!("Connecting to SurrealDB...");
    let surreal = SurrealStorage::connect().await
        .map_err(|e| anyhow::anyhow!("SurrealDB connect failed: {e}"))?;
    surreal.init_schema().await
        .map_err(|e| anyhow::anyhow!("SurrealDB schema init failed: {e}"))?;
    tracing::info!("SurrealDB ready.");

    // ── Embedding model ───────────────────────────────────────────────────────
    // Load BGE-small once. This takes a few seconds on first boot
    // (downloads model weights if not cached locally).
    tracing::info!("Loading BGE-small embedding model...");
    let embedder = EmbeddingContext::new()
        .map_err(|e| anyhow::anyhow!("Embedding model failed to load: {e}"))?;
    tracing::info!("Embedding model ready.");

    // ── State ─────────────────────────────────────────────────────────────────
    let state = AppState {
        surreal:  Arc::new(surreal),
        embedder: Arc::new(RwLock::new(embedder)),
        registry: Arc::new(RwLock::new(GraphRegistry::new())),
    };

    // ── CORS ──────────────────────────────────────────────────────────────────
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    // ── Router ────────────────────────────────────────────────────────────────
    // GraphQL subgraph endpoint added Day 5 when Apollo Router is wired.
    // Health check lets docker-compose verify the service is up.
    let app = Router::new()
        .route("/health", get(health))
        .layer(cors)
        .with_state(state);

    // ── Listen ────────────────────────────────────────────────────────────────
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("fluvio-graph listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
}