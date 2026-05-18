//! HTTP client for fluvio-ingestion.
//! File processing and embedding goes through here.

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct IngestResult {
    pub status:  String,
    pub message: String,
    pub job_id:  String,
}

#[derive(Clone)]
pub struct IngestionClient {
    pub endpoint: String,
    pub client:   Client,
}

impl IngestionClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), client: Client::new() }
    }

    /// Ingest raw text — returns immediately with status.
    pub async fn ingest_raw(
        &self,
        owner_id:   Uuid,
        text:       &str,
        source_uri: &str,
        domain:     &str,
    ) -> anyhow::Result<IngestResult> {
        let q = r#"mutation($text: String!, $sourceUri: String!, $domain: String) {
            ingestRaw(text: $text, sourceUri: $sourceUri, domain: $domain) {
                jobId status message
            }
        }"#;

        let body = self.post(owner_id, q, json!({
            "text":      text,
            "sourceUri": source_uri,
            "domain":    domain,
        })).await?;

        let r = &body["data"]["ingestRaw"];
        Ok(IngestResult {
            job_id:  r["jobId"].as_str().unwrap_or("").to_string(),
            status:  r["status"].as_str().unwrap_or("").to_string(),
            message: r["message"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Check status of an async ingestion job.
    pub async fn get_job_status(
        &self,
        owner_id: Uuid,
        job_id:   &str,
    ) -> anyhow::Result<Option<String>> {
        let q = r#"query($jobId: String!) {
            ingestJob(jobId: $jobId) { status chunkCount }
        }"#;

        let body = self.post(owner_id, q, json!({ "jobId": job_id })).await?;
        Ok(body["data"]["ingestJob"]["status"].as_str().map(String::from))
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    async fn post(&self, owner_id: Uuid, query: &str, variables: Value) -> anyhow::Result<Value> {
        let resp = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-user-id", owner_id.to_string())
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .context("failed to reach fluvio-ingestion")?;

        let body: Value = resp.json().await
            .context("failed to parse fluvio-ingestion response")?;

        if let Some(errors) = body.get("errors") {
            anyhow::bail!("fluvio-ingestion error: {errors}");
        }

        Ok(body)
    }
}