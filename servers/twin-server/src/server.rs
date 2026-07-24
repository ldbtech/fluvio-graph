//! Axum server for fluvio-twin.

use axum::{Router, routing::get, http::Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use fluvio_twin_core::graph::GraphClient;
use crate::graphql::{build_schema, graphql_router};

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// GraphQL client → fluvio-graph
    pub graph_client:  GraphClient,
    /// Anthropic API key for Claude
    pub anthropic_key: String,
}

// ── serve() ───────────────────────────────────────────────────────────────────

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3002".to_string());

    let graph_url = std::env::var("GRAPH_GRAPHQL_URL")
        .unwrap_or_else(|_| "http://localhost:3001/graphql".to_string());

    let anthropic_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!(
            "ANTHROPIC_API_KEY not set — add it to .env or export it"
        ))?;

    let graph_client = GraphClient::new(&graph_url);
    tracing::info!("Graph client → {graph_url}");

    let state = AppState { graph_client, anthropic_key };

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

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("fluvio-twin listening on http://{addr}");
    tracing::info!("GraphQL endpoint: http://{addr}/graphql");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str { "ok" }