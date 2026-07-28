//! Axum server for fluvio-twin.

use axum::{Router, routing::get, http::Method};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use fluvio_twin_core::graph::GraphClient;
use fluvio_llm::resolver::CredentialResolver;
use crate::graphql::{build_schema, graphql_router};

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    /// GraphQL client → fluvio-graph
    pub graph_client: GraphClient,
    /// Resolves the caller's LLM provider connection (or deployment
    /// fallback) from fluvio-database's internal credential route.
    pub llm_resolver: CredentialResolver,
}

// ── serve() ───────────────────────────────────────────────────────────────────

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3002".to_string());

    let graph_url = std::env::var("GRAPH_GRAPHQL_URL")
        .unwrap_or_else(|_| "http://localhost:3001/graphql".to_string());

    // Bare base URL (not /graphql-suffixed) — this hits fluvio-database's
    // plain internal credential-resolution route, not its GraphQL endpoint.
    let database_service_url = std::env::var("DATABASE_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:3005".to_string());
    let internal_secret = std::env::var("FLUVIOME_INTERNAL_SECRET").ok()
        .filter(|s| !s.trim().is_empty());

    let graph_client = GraphClient::new(&graph_url);
    tracing::info!("Graph client → {graph_url}");
    tracing::info!("Database credential resolver → {database_service_url}");

    let llm_resolver = CredentialResolver::new(database_service_url, internal_secret);
    let state = AppState { graph_client, llm_resolver };

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