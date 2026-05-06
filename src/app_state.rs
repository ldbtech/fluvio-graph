//! Shared HTTP server state (extracted so route modules can depend on it without cycles with `server`).

use crate::agents::tool_spawner::{JobStore, ToolSpawner};
use crate::graph::structs::DomainGraph;
use crate::ingestion::IngestionPipeline;
use crate::ingestion_registry::architecture::SpaceProgram;
use crate::ingestion_registry::email::GmailSyncProgress;
use crate::agent_jobs::AgentStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sqlx::PgPool;

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
    pub oauth_csrf: Arc<Mutex<Option<String>>>,
    pub gmail_progress: Arc<GmailSyncProgress>,
    pub presist: fn(&DomainGraph) -> anyhow::Result<()>,
    pub agent_store: AgentStore,
    pub architecture_designs: Arc<Mutex<HashMap<String, SpaceProgram>>>,
    pub tool_spawner: Arc<ToolSpawner>,
    pub job_store: JobStore,
    
    pub pg_pool: PgPool,
    
    pub fluvio_mock_docs: Arc<Mutex<Vec<FluvioMockDocument>>>,
    pub fluvio_account_profile: Arc<Mutex<FluvioAccountProfile>>,
}
