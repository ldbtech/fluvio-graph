//! HTTP client for fluvio-ingestion.

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct IngestResult {
    pub status:  String,
    pub message: String,
    pub job_id:  String,
    /// Node ID written to fluvio-graph (if synchronous)
    pub node_id: Option<String>,
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

    /// Ingest raw text for a personal twin (no group).
    pub async fn ingest_raw(
        &self,
        owner_id:   Uuid,
        text:       &str,
        source_uri: &str,
        domain:     &str,
    ) -> anyhow::Result<IngestResult> {
        self.ingest_raw_inner(owner_id, text, source_uri, domain, None, None).await
    }

    /// Ingest raw text for a collab group.
    /// group_id and status are stored as metadata on the node.
    pub async fn ingest_raw_for_group(
        &self,
        owner_id:   Uuid,
        text:       &str,
        source_uri: &str,
        group_id:   &str,
        status:     &str,
    ) -> anyhow::Result<IngestResult> {
        self.ingest_raw_inner(
            owner_id, text, source_uri, "custom",
            Some(group_id), Some(status),
        ).await
    }

    async fn ingest_raw_inner(
        &self,
        owner_id:   Uuid,
        text:       &str,
        source_uri: &str,
        domain:     &str,
        group_id:   Option<&str>,
        status:     Option<&str>,
    ) -> anyhow::Result<IngestResult> {
        // Encode group metadata into source_uri so the node is tagged
        // Format: original_uri|group=GROUP_ID|status=STATUS
        let tagged_uri = match (group_id, status) {
            (Some(gid), Some(s)) =>
                format!("{source_uri}|group={gid}|status={s}"),
            _ => source_uri.to_string(),
        };

        let q = r#"mutation($text: String!, $sourceUri: String!, $domain: String) {
            ingestRaw(text: $text, sourceUri: $sourceUri, domain: $domain) {
                jobId status message
            }
        }"#;

        let body = self.post(owner_id, q, json!({
            "text":      text,
            "sourceUri": tagged_uri,
            "domain":    domain,
        })).await?;

        let r = &body["data"]["ingestRaw"];
        Ok(IngestResult {
            job_id:  r["jobId"].as_str().unwrap_or("").to_string(),
            status:  r["status"].as_str().unwrap_or("").to_string(),
            message: r["message"].as_str().unwrap_or("").to_string(),
            node_id: None,
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

    async fn post(
        &self,
        owner_id:  Uuid,
        query:     &str,
        variables: Value,
    ) -> anyhow::Result<Value> {
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