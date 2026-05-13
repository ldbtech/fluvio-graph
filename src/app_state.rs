//! Shared HTTP server state (extracted so route modules can depend on it without cycles with `server`).
use crate::agent_jobs::AgentStore;
use crate::ingestion::IngestionPipeline;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// SurrealDB Storage.
use crate::storage::surreal::SurrealStorage;

/// CSRF token + owning Fluvio user for the in-flight Gmail browser OAuth handshake.
#[derive(Clone)]
pub struct GmailOauthPending {
    pub csrf_state: String,
    pub user_id:    Uuid,
}

/// Contact fields for the Fluvio NFC owner profile (`POST /fluvio/mock/account/profile`).
#[derive(Clone, Default)]
pub struct FluvioAccountProfile {
    pub email: String,
    pub phone: String,
}

/// User-uploaded documents for the Fluvio personal dashboard (`POST /fluvio/mock/ingest`).
#[derive(Clone, Serialize, Deserialize)]
pub struct FluvioMockDocument {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub excerpt: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pipeline: Arc<Mutex<IngestionPipeline>>,
    pub api_key: String,
    pub oauth_gmail: Arc<Mutex<Option<GmailOauthPending>>>,
    pub agent_store: AgentStore,

    pub pg_pool: PgPool,

    pub fluvio_mock_docs: Arc<Mutex<Vec<FluvioMockDocument>>>,
    pub fluvio_account_profile: Arc<Mutex<FluvioAccountProfile>>,

    /// Durable graph store. Codebase, PDF, video, email ingest all persist through this.
    pub surreal_storage: Arc<SurrealStorage>,
}
