//! Axum server for fluvio-collab.

use axum::{Router, routing::get, http::Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::clients::{DatabaseClient, GraphClient, IngestionClient};
use crate::graphql::{build_schema, graphql_router};

#[derive(Clone)]
pub struct AppState {
    pub db:            DatabaseClient,
    pub graph:         GraphClient,
    pub ingestion:     IngestionClient,
    pub anthropic_key: String,
}

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("FLUVIO_COLLAB_PORT")
        .or_else(|_| std::env::var("PORT"))
        .unwrap_or_else(|_| "3003".to_string());

    let db_url = std::env::var("DATABASE_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3005/graphql".to_string());

    let graph_url = std::env::var("GRAPH_GRAPHQL_URL")
        .unwrap_or_else(|_| "http://localhost:3001/graphql".to_string());

    let ingestion_url = std::env::var("INGESTION_GRAPHQL_URL")
        .unwrap_or_else(|_| "http://localhost:3004/graphql".to_string());

    let anthropic_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    tracing::info!("Database service → {db_url}");
    tracing::info!("Graph service    → {graph_url}");
    tracing::info!("Ingestion service→ {ingestion_url}");

    let state = AppState {
        db:            DatabaseClient::new(&db_url),
        graph:         GraphClient::new(&graph_url),
        ingestion:     IngestionClient::new(&ingestion_url),
        anthropic_key,
    };

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
    tracing::info!("fluvio-collab listening on http://{addr}");
    tracing::info!("GraphQL endpoint: http://{addr}/graphql");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str { "ok" }