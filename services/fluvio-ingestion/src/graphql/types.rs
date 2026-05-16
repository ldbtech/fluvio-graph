//! GraphQL types for fluvio-ingestion subgraph.

use async_graphql::*;
use crate::pipeline::job_store::IngestJob;

#[derive(SimpleObject, Clone, Debug)]
pub struct GqlIngestResult {
    pub job_id:      String,
    pub status:      String,
    pub message:     String,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct GqlIngestJob {
    pub id:          String,
    pub owner_id:    String,
    pub filename:    String,
    pub status:      String,
    pub chunk_count: i32,
    pub node_ids:    Vec<String>,
    pub error:       Option<String>,
}

impl From<IngestJob> for GqlIngestJob {
    fn from(j: IngestJob) -> Self {
        Self {
            id:          j.id,
            owner_id:    j.owner_id,
            filename:    j.filename,
            status:      j.status.to_string(),
            chunk_count: j.chunk_count as i32,
            node_ids:    j.node_ids,
            error:       j.error,
        }
    }
}