//! Axum server for fluvio-graph.

use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    http::{Method, HeaderMap},
    middleware::{self, Next},
    extract::Request,
    response::Response,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tokio::sync::RwLock;

use crate::storage::surreal::{SurrealConfig, SurrealStorage};
use crate::registry::GraphRegistry;
use crate::embeddings::EmbeddingContext;
use crate::graphql::{build_schema, graphql_router, extract_user_id_from_headers};

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub surreal:  Arc<SurrealStorage>,
    pub embedder: Arc<RwLock<EmbeddingContext>>,
    pub registry: Arc<RwLock<GraphRegistry>>,
}

// ── serve() ───────────────────────────────────────────────────────────────────

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());

    tracing::info!("Connecting to SurrealDB...");
    let surreal = SurrealStorage::connect(&SurrealConfig::from_env()).await
        .map_err(|e| anyhow::anyhow!("SurrealDB connect failed: {e}"))?;
    surreal.init_schema().await
        .map_err(|e| anyhow::anyhow!("SurrealDB schema init failed: {e}"))?;
    tracing::info!("SurrealDB ready.");

    tracing::info!("Loading BGE-small embedding model...");
    let embedder = EmbeddingContext::new()
        .map_err(|e| anyhow::anyhow!("Embedding model failed to load: {e}"))?;
    tracing::info!("Embedding model ready.");

    let state = AppState {
        surreal:  Arc::new(surreal),
        embedder: Arc::new(RwLock::new(embedder)),
        registry: Arc::new(RwLock::new(GraphRegistry::new())),
    };

    let schema = build_schema(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .merge(graphql_router(schema))
        .layer(middleware::from_fn(user_id_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("fluvio-graph listening on http://{addr}");
    tracing::info!("GraphQL endpoint: http://{addr}/graphql");
    axum::serve(listener, app).await?;

    Ok(())
}

// ── Middleware ────────────────────────────────────────────────────────────────

async fn user_id_middleware(
    headers: HeaderMap,
    mut req: Request,
    next:    Next,
) -> Response {
    if let Some(user_id) = extract_user_id_from_headers(&headers) {
        req.extensions_mut().insert(user_id);
    }
    next.run(req).await
}

async fn health() -> &'static str { "ok" }