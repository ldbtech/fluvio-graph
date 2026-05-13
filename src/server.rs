use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::Method,
    middleware,
    routing::{get, post},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use crate::{
    graph::enums::Domain,
    graph::structs::{DomainGraph, GraphId},
    graph::EmbeddingContext,
    ingestion::IngestionPipeline,
};

use crate::routes::kg_chat::post_kg_chat;
use crate::routes::rules::{
    post_rules_link, post_security_deploy,
    get_security_status, get_security_result,
};
use crate::routes::codebase::{
    get_codebase_parse, get_codebase_tree, post_codebase_clone, post_codebase_ingest, post_codebase_resolve,
};
use crate::ingestion_registry::email::routes::gmail_router;
use crate::ingestion_registry::documents::pdf::routes::{pdf_ingest_router, user_uploads_router};

use crate::authentication::{
    multipart_upload_must_be_logged_in, require_logged_in_session,
};

// SurrealDB Storage.
use crate::storage::surreal::SurrealStorage;

// Video routes
use crate::routes::video::{
    post_ingest_video, get_video,
    get_video_scenes, get_video_status,
};

pub use crate::app_state::AppState;
use crate::database::pool::setup_database;

pub async fn serve(api_key: String) -> anyhow::Result<()> {
    // Postgres first so migration failures do not wait for ONNX / embedding model load.
    let pg_pool = setup_database().await?;

    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let graph = DomainGraph::new(GraphId::new("workspace"), Domain::Custom("workspace".into()));

    println!(
        "Workspace RAM graph is a per-process working set; durable chunks live in SurrealDB."
    );

    let pipeline = Arc::new(Mutex::new(IngestionPipeline::new(graph, embed_ctx.clone())));

    // All origins OK for dev; include full method set used by routers (PUT /twin/zone,
    // DELETE /tools/jobs/…, OPTIONS preflight).
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
        ])
        .allow_headers(Any);

    // SurrealDB Storage.
    let surreal_storage = SurrealStorage::connect().await
        .map_err(|e| anyhow::anyhow!("Failed to connect to SurrealDB: {e}"))?;

    surreal_storage.init_schema().await
        .map_err(|e| anyhow::anyhow!("Failed to init SurrealDB schema: {e}"))?;

    let state = AppState {
        pipeline,
        api_key,
        oauth_gmail: Arc::new(Mutex::new(None)),
        agent_store:  Arc::new(Mutex::new(HashMap::new())),

        pg_pool,

        fluvio_mock_docs:       Arc::new(Mutex::new(Vec::new())),
        fluvio_account_profile: Arc::new(Mutex::new(
            crate::app_state::FluvioAccountProfile::default(),
        )),

        surreal_storage: Arc::new(surreal_storage),
    };

    let agent_poll_state = state.clone();
    tokio::spawn(async move {
        let secs: u64 = std::env::var("GMAIL_AGENT_AUTO_POLL_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(45)
            .clamp(15, 600);
        tracing::info!(
            "[kg-engine] Gmail reply agent inbox pass every {}s \
             (GMAIL_AGENT_AUTO_POLL_INTERVAL_SECS; all users with Gmail OAuth)",
            secs
        );
        let mut ticker = tokio::time::interval(Duration::from_secs(secs));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            crate::ingestion_registry::email::reply_agent::run_gmail_agent_auto_poll_tick(
                &agent_poll_state,
            )
            .await;
        }
    });

    let ingest_upload = Router::<AppState>::new()
        //// PDF (`ingestion_registry/documents/pdf/routes.rs`) — stamps upload user header ////
        .merge(pdf_ingest_router())
        .route("/ingest/video", post(post_ingest_video))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            multipart_upload_must_be_logged_in,
        ));

    // Everything requires `Authorization: Bearer` except OTP bootstrap routes (merged separately below)
    // and paths listed in `route_allows_anonymous` inside `require_logged_in_session` (e.g. Gmail browser OAuth).
    let protected_api = Router::<AppState>::new()
        //// Ingest routes (upload library: `ingestion_registry/documents/pdf/routes.rs`) ////
        .route("/chat", post(post_kg_chat))
        //// Codebase routes ////
        .route("/ingest", post(post_codebase_ingest))
        .route("/codebase/clone", post(post_codebase_clone))
        .route("/sync/codebase/clone", post(post_codebase_clone))
        .route("/parse", get(get_codebase_parse))
        .route("/tree", get(get_codebase_tree))
        .route("/codebase/resolve", post(post_codebase_resolve))
        .route("/rules/link", post(post_rules_link))
        .route("/agents/security/deploy", post(post_security_deploy))
        .route("/agents/security/{id}/status", get(get_security_status))
        .route("/agents/security/{id}/result", get(get_security_result))
        .route("/video/{id}", get(get_video))
        .route("/video/{id}/scenes", get(get_video_scenes))
        .route("/video/{id}/status", get(get_video_status))
        
        .merge(ingest_upload)
        .merge(user_uploads_router())
        .merge(gmail_router())
        .merge(crate::routes::twin::routes())  
        .merge(crate::routes::auth::authenticated_auth_routes())
        
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_logged_in_session,
        ));

    let app = Router::<AppState>::new()
        .merge(crate::routes::auth::public_auth_routes())
        .merge(crate::routes::twin::public_onboarding_routes())
        .merge(protected_api)
        .layer(DefaultBodyLimit::max(256 * 1024 * 1024))
        .layer(cors)
        .with_state(state);

    let listner = tokio::net::TcpListener::bind("0.0.0.0:8001").await?;

    println!("KG-GRAPH Listening on http://localhost:8001");
    axum::serve(listner, app).await?;

    Ok(())
}