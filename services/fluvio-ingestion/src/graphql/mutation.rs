//! Ingestion mutations.

use async_graphql::*;
use uuid::Uuid;
use fluvio_types::Domain;

use crate::server::AppState;
use crate::graphql::types::GqlIngestResult;

pub struct MutationRoot;

#[Object(name = "Mutation")]
impl MutationRoot {

    /// Upload and ingest a file (PDF, TXT, MD).
    /// Returns immediately with a jobId — processing runs in background.
    async fn ingest_file(
        &self,
        ctx:      &Context<'_>,
        file:     Upload,
        filename: String,
        zone:     Option<i32>,
    ) -> Result<GqlIngestResult> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let zone    = zone.unwrap_or(0) as i16;

        let upload  = file.value(ctx)?;
        let bytes   = tokio::fs::read(&upload.filename)
            .await
            .unwrap_or_else(|_| upload.filename.as_bytes().to_vec());

        let job_id = state.pipeline
            .ingest_pdf_async(user_id, bytes, filename.clone(), zone)
            .await;

        tracing::info!(
            user_id = %user_id,
            filename = %filename,
            job_id  = %job_id,
            "ingestion job queued"
        );

        Ok(GqlIngestResult {
            job_id,
            status:  "queued".into(),
            message: format!("ingestion started for {filename}"),
        })
    }

    /// Ingest raw text directly — no file upload needed.
    /// Runs synchronously since text is small.
    async fn ingest_raw(
        &self,
        ctx:        &Context<'_>,
        text:       String,
        source_uri: String,
        domain:     Option<String>,
        zone:       Option<i32>,
    ) -> Result<GqlIngestResult> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let zone    = zone.unwrap_or(0) as i16;
        let domain  = parse_domain(domain.as_deref());

        let result = state.pipeline
            .ingest_text(user_id, text, source_uri, domain, zone)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlIngestResult {
            job_id:  String::new(),
            status:  "complete".into(),
            message: format!("{} chunks ingested", result.chunk_count),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing"))
}

fn parse_domain(s: Option<&str>) -> Domain {
    match s {
        Some("pdf")      => Domain::Pdf,
        Some("email")    => Domain::Email,
        Some("codebase") => Domain::Codebase,
        Some("web")      => Domain::Web,
        Some(other)      => Domain::Custom(other.to_string()),
        None             => Domain::Custom("raw".into()),
    }
}