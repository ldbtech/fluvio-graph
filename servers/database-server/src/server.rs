//! Axum server for fluvio-database.

use axum::{Router, routing::get, http::Method};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use fluvio_database::db::pool::{create_pool, run_migrations};
use fluvio_llm::crypto::CredentialKey;
use crate::graphql::{build_schema, graphql_router};
use crate::internal;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub company_pool: PgPool,
    /// AES-256-GCM master key for BYOK credential encryption. `None` if
    /// `FLUVIOME_CREDENTIAL_KEY` is unset/malformed — this is a graceful
    /// degrade, not a boot failure: BYOK connect/resolve operations just
    /// error until it's configured.
    pub credential_key: Option<CredentialKey>,
    /// Optional shared secret guarding `/internal/resolve-llm-credential`.
    /// `None` = open (matches this deployment's `x-user-id`-only trust model).
    pub internal_secret: Option<String>,
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

    let credential_key = match std::env::var("FLUVIOME_CREDENTIAL_KEY") {
        Ok(raw) if !raw.trim().is_empty() => match CredentialKey::from_base64(&raw) {
            Ok(key) => Some(key),
            Err(e) => {
                tracing::warn!("FLUVIOME_CREDENTIAL_KEY is set but invalid ({e}) — \
                    BYOK connect/resolve operations will error until it's fixed");
                None
            }
        },
        _ => {
            tracing::warn!("FLUVIOME_CREDENTIAL_KEY not set — BYOK connect/resolve \
                operations will error until it's configured");
            None
        }
    };

    let internal_secret = std::env::var("FLUVIOME_INTERNAL_SECRET").ok()
        .filter(|s| !s.trim().is_empty());

    // Connect + migrate
    let pool = create_pool(&database_url).await?;
    run_migrations(&pool).await?;

    let company_pool = create_pool(&company_database_url).await?;
    run_migrations(&company_pool).await?;

    let state  = AppState { pool, company_pool, credential_key, internal_secret };
    let schema = build_schema(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .merge(internal::router(state.clone()))
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