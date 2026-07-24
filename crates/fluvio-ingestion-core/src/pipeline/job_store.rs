//! In-memory job registry for async ingestion jobs.
//!
//! When a file is uploaded, an IngestJob is created immediately and
//! returned to the client. The actual processing runs in the background.
//! The client polls GET /ingest/job/{id} to check progress.
//!
//! Jobs are stored in a DashMap (lock-free concurrent HashMap).
//! They are not persisted — if the service restarts, jobs are lost.
//! This is acceptable for now; add Redis persistence when needed.

use std::sync::Arc;
use std::time::Instant;
use dashmap::DashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── JobStatus ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Complete,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Queued   => write!(f, "queued"),
            JobStatus::Running  => write!(f, "running"),
            JobStatus::Complete => write!(f, "complete"),
            JobStatus::Failed   => write!(f, "failed"),
        }
    }
}

// ── IngestJob ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestJob {
    pub id:          String,
    pub owner_id:    String,
    pub filename:    String,
    pub status:      JobStatus,
    pub chunk_count: usize,
    pub node_ids:    Vec<String>,
    pub error:       Option<String>,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

impl IngestJob {
    pub fn new(owner_id: Uuid, filename: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id:          Uuid::new_v4().to_string(),
            owner_id:    owner_id.to_string(),
            filename:    filename.into(),
            status:      JobStatus::Queued,
            chunk_count: 0,
            node_ids:    vec![],
            error:       None,
            created_at:  now,
            updated_at:  now,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.status, JobStatus::Complete | JobStatus::Failed)
    }
}

// ── JobStore ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct JobStore {
    jobs: Arc<DashMap<String, IngestJob>>,
}

impl JobStore {
    pub fn new() -> Self {
        Self { jobs: Arc::new(DashMap::new()) }
    }

    pub fn insert(&self, job: IngestJob) {
        self.jobs.insert(job.id.clone(), job);
    }

    pub fn get(&self, id: &str) -> Option<IngestJob> {
        self.jobs.get(id).map(|j| j.clone())
    }

    pub fn update_status(&self, id: &str, status: JobStatus) {
        if let Some(mut job) = self.jobs.get_mut(id) {
            job.status     = status;
            job.updated_at = Utc::now();
        }
    }

    pub fn complete(&self, id: &str, node_ids: Vec<String>, chunk_count: usize) {
        if let Some(mut job) = self.jobs.get_mut(id) {
            job.status      = JobStatus::Complete;
            job.node_ids    = node_ids;
            job.chunk_count = chunk_count;
            job.updated_at  = Utc::now();
        }
    }

    pub fn fail(&self, id: &str, error: impl Into<String>) {
        if let Some(mut job) = self.jobs.get_mut(id) {
            job.status     = JobStatus::Failed;
            job.error      = Some(error.into());
            job.updated_at = Utc::now();
        }
    }

    /// Remove jobs older than `max_age_secs` to prevent unbounded memory growth.
    pub fn evict_old_jobs(&self, max_age_secs: u64) {
        let cutoff = Utc::now() - chrono::Duration::seconds(max_age_secs as i64);
        self.jobs.retain(|_, job| job.updated_at > cutoff);
    }
}

impl Default for JobStore {
    fn default() -> Self { Self::new() }
}