//! In-memory security agent job registry (shared by `AppState` and rules routes).

use crate::ingestion_registry::documents::rule_linker::security_agent::{
    SecurityAgentProgress, SecurityAgentResult,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct AgentJobEntry {
    pub progress: Arc<SecurityAgentProgress>,
    pub result: Arc<Mutex<Option<SecurityAgentResult>>>,
}

pub type AgentStore = Arc<Mutex<HashMap<String, AgentJobEntry>>>;
