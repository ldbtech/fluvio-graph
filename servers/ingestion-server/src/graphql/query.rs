//! Query resolvers for fluvio-ingestion.

use async_graphql::*;
use crate::server::AppState;
use crate::graphql::types::GqlIngestJob;
use crate::graphql::mutation::extract_user_id;

pub struct QueryRoot;

#[Object(name = "Query")]
impl QueryRoot {
    /// Get the status of an ingestion job by ID.
    async fn ingest_job(
        &self,
        ctx:    &Context<'_>,
        job_id: String,
    ) -> Result<Option<GqlIngestJob>> {
        let state   = ctx.data::<AppState>()?;
        let _user   = extract_user_id(ctx)?;

        let job = state.pipeline.job_store.get(&job_id)
            .map(GqlIngestJob::from);

        Ok(job)
    }
}