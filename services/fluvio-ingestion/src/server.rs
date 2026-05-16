//! Axum server for fluvio-ingestion.

use std::sync::Arc;
use axum::{Router, routing::get, http::Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::client::GraphClient;
use crate::graphql::{build_schema, graphql_router};
use crate::pipeline::{embedder::Embedder, IngestionPipeline};

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub pipeline: IngestionPipeline,
}

// ── serve() ───────────────────────────────────────────────────────────────────

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3004".to_string());

    let graph_url = std::env::var("GRAPH_GRAPHQL_URL")
        .unwrap_or_else(|_| "http://localhost:3001/graphql".to_string());

    // ── Embedding model ───────────────────────────────────────────────────────
    tracing::info!("Loading BGE-small embedding model...");
    let embedder = Embedder::new()
        .map_err(|e| anyhow::anyhow!("Embedding model failed: {e}"))?;
    tracing::info!("Embedding model ready.");

    // ── Graph client ──────────────────────────────────────────────────────────
    let graph_client = GraphClient::new(&graph_url);
    tracing::info!("Graph client configured → {graph_url}");

    // ── Pipeline ──────────────────────────────────────────────────────────────
    let pipeline = IngestionPipeline::new(embedder, graph_client);

    let state = AppState { pipeline };

    // ── Job eviction background task ──────────────────────────────────────────
    // Evict completed jobs older than 1 hour every 5 minutes
    let job_store = state.pipeline.job_store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            std::time::Duration::from_secs(300)
        );
        loop {
            interval.tick().await;
            job_store.evict_old_jobs(3600);
        }
    });

    // ── Schema + router ───────────────────────────────────────────────────────
    let schema = build_schema(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .merge(graphql_router(schema))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // ── Listen ────────────────────────────────────────────────────────────────
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("fluvio-ingestion listening on http://{addr}");
    tracing::info!("GraphQL endpoint: http://{addr}/graphql");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str { "ok" }