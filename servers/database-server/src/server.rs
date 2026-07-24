//! Axum server for fluvio-database.

use axum::{Router, routing::get, http::Method};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use fluvio_database::db::pool::{create_pool, run_migrations};
use crate::graphql::{build_schema, graphql_router};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub company_pool: PgPool,
}

pub async fn serve() -> anyhow::Result<()> {
    let port = std::env::var("FLUVIO_DATABASE_PORT")
        .or_else(|_| std::env::var("PORT"))
        .unwrap_or_else(|_| "3005".to_string());

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!(
            "DATABASE_URL not set — e.g. postgres://localhost/fluvio_collab"
        ))?;

    let company_database_url = std::env::var("COMPANY_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/fluvio_company".to_string());

    // Connect + migrate
    let pool = create_pool(&database_url).await?;
    run_migrations(&pool).await?;

    let company_pool = create_pool(&company_database_url).await?;
    run_migrations(&company_pool).await?;

    let state  = AppState { pool, company_pool };
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
    tracing::info!("fluvio-database listening on http://{addr}");
    tracing::info!("GraphQL endpoint: http://{addr}/graphql");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str { "ok" }