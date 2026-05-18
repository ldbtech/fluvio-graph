//! HTTP client for fluvio-graph.
//! All graph storage and search goes through here.

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id:          String,
    pub source_text: String,
    pub source_uri:  String,
    pub domain:      String,
    pub kind:        String,
    pub score:       f32,
    pub metadata:    Vec<(String, String)>,
}

#[derive(Clone)]
pub struct GraphClient {
    pub endpoint: String,
    pub client:   Client,
}

impl GraphClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), client: Client::new() }
    }

    /// Upsert a node tagged with group_id and status.
    /// group_id and status go into node metadata.
    pub async fn upsert_group_node(
        &self,
        owner_id:   Uuid,
        source_text: &str,
        source_uri:  &str,
        group_id:    &str,
        contributed_by: &str,
        status:      &str,
        zone:        i32,
    ) -> anyhow::Result<String> {
        let q = r#"mutation($input: GqlNodeInput!) {
            upsertNode(input: $input) { id }
        }"#;

        let body = self.post(owner_id, q, json!({
            "input": {
                "domain":     "CUSTOM",
                "sourceUri":  source_uri,
                "sourceText": source_text,
                "kind":       "ARTIFCAT",
                "zone":       zone,
                "metadata": [
                    { "key": "group_id",       "value": group_id },
                    { "key": "contributed_by", "value": contributed_by },
                    { "key": "status",         "value": status },
                ]
            }
        })).await?;

        body["data"]["upsertNode"]["id"]
            .as_str()
            .map(String::from)
            .context("upsertNode returned no id")
    }

    /// Update node status metadata (pending → approved | rejected).
    pub async fn update_node_status(
        &self,
        owner_id: Uuid,
        node_id:  &str,
        status:   &str,
    ) -> anyhow::Result<()> {
        // Fetch existing node, update status metadata, re-upsert
        let node = self.get_node(owner_id, node_id).await?
            .context("node not found")?;

        let q = r#"mutation($input: GqlNodeInput!) {
            upsertNode(input: $input) { id }
        }"#;

        let mut metadata: Vec<Value> = node.metadata.iter()
            .filter(|(k, _)| k != "status")
            .map(|(k, v)| json!({ "key": k, "value": v }))
            .collect();
        metadata.push(json!({ "key": "status", "value": status }));

        self.post(owner_id, q, json!({
            "input": {
                "id":         node_id,
                "domain":     "CUSTOM",
                "sourceUri":  node.source_uri,
                "sourceText": node.source_text,
                "kind":       "ARTIFCAT",
                "zone":       0,
                "metadata":   metadata,
            }
        })).await?;

        Ok(())
    }

    /// Semantic search within a group — approved nodes only.
    pub async fn search_group(
        &self,
        owner_id: Uuid,
        query:    &str,
        group_id: &str,
        top_k:    usize,
    ) -> anyhow::Result<Vec<GraphNode>> {
        let q = r#"query($query: String!, $config: GqlQueryConfig) {
            search(query: $query, config: $config) {
                score
                node { id sourceText domain kind metadata { key value } }
            }
        }"#;

        let body = self.post(owner_id, q, json!({
            "query":  query,
            "config": { "similarityTopK": top_k as i32, "maxZone": 0 }
        })).await?;

        let results = body["data"]["search"]
            .as_array().cloned().unwrap_or_default();

        // Filter to only approved nodes in this group
        Ok(results.into_iter().filter_map(|r| {
            let score = r["score"].as_f64().unwrap_or(0.0) as f32;
            let n = &r["node"];
            let meta: Vec<(String, String)> = n["metadata"]
                .as_array().cloned().unwrap_or_default()
                .into_iter()
                .filter_map(|m| Some((
                    m["key"].as_str()?.to_string(),
                    m["value"].as_str()?.to_string(),
                )))
                .collect();

            // Only return approved nodes belonging to this group
            let node_group  = meta.iter().find(|(k, _)| k == "group_id").map(|(_, v)| v.as_str());
            let node_status = meta.iter().find(|(k, _)| k == "status").map(|(_, v)| v.as_str());

            if node_group != Some(group_id) || node_status != Some("approved") {
                return None;
            }

            Some(GraphNode {
                id:          n["id"].as_str()?.to_string(),
                source_text: n["sourceText"].as_str()?.to_string(),
                source_uri:  String::new(),
                domain:      n["domain"].as_str()?.to_string(),
                kind:        n["kind"].as_str()?.to_string(),
                metadata:    meta,
                score,
            })
        }).collect())
    }

    /// BFS neighbors of a node.
    pub async fn neighbors(
        &self,
        owner_id: Uuid,
        node_id:  &str,
        depth:    usize,
    ) -> anyhow::Result<Vec<GraphNode>> {
        let q = r#"query($id: String!, $depth: Int) {
            neighbors(id: $id, depth: $depth) {
                id sourceText domain kind metadata { key value }
            }
        }"#;

        let body = self.post(owner_id, q, json!({
            "id": node_id, "depth": depth as i32
        })).await?;

        Ok(body["data"]["neighbors"]
            .as_array().cloned().unwrap_or_default()
            .into_iter()
            .filter_map(|n| Some(GraphNode {
                id:          n["id"].as_str()?.to_string(),
                source_text: n["sourceText"].as_str()?.to_string(),
                source_uri:  String::new(),
                domain:      n["domain"].as_str()?.to_string(),
                kind:        n["kind"].as_str()?.to_string(),
                metadata:    n["metadata"].as_array().cloned().unwrap_or_default()
                    .into_iter()
                    .filter_map(|m| Some((
                        m["key"].as_str()?.to_string(),
                        m["value"].as_str()?.to_string(),
                    )))
                    .collect(),
                score: 0.0,
            }))
            .collect())
    }

    /// Get a single node by ID.
    pub async fn get_node(
        &self,
        owner_id: Uuid,
        node_id:  &str,
    ) -> anyhow::Result<Option<GraphNode>> {
        let q = r#"query($id: String!) {
            node(id: $id) {
                id sourceText domain kind metadata { key value }
            }
        }"#;

        let body = self.post(owner_id, q, json!({ "id": node_id })).await?;
        let n = &body["data"]["node"];
        if n.is_null() { return Ok(None); }

        Ok(Some(GraphNode {
            id:          n["id"].as_str().unwrap_or("").to_string(),
            source_text: n["sourceText"].as_str().unwrap_or("").to_string(),
            source_uri:  String::new(),
            domain:      n["domain"].as_str().unwrap_or("").to_string(),
            kind:        n["kind"].as_str().unwrap_or("").to_string(),
            metadata:    n["metadata"].as_array().cloned().unwrap_or_default()
                .into_iter()
                .filter_map(|m| Some((
                    m["key"].as_str()?.to_string(),
                    m["value"].as_str()?.to_string(),
                )))
                .collect(),
            score: 0.0,
        }))
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    async fn post(&self, owner_id: Uuid, query: &str, variables: Value) -> anyhow::Result<Value> {
        let resp = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-user-id", owner_id.to_string())
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .context("failed to reach fluvio-graph")?;

        let body: Value = resp.json().await
            .context("failed to parse fluvio-graph response")?;

        if let Some(errors) = body.get("errors") {
            anyhow::bail!("fluvio-graph error: {errors}");
        }

        Ok(body)
    }
}