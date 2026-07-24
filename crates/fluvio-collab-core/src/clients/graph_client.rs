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

impl GraphNode {
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.metadata.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
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

    /// Find the node that ingestion just created (by source_text match),
    /// then update its metadata to add group_id + contributed_by + status.
    ///
    /// This is the key collab operation:
    ///   ingestion creates an embedded node → we tag it for the group
    ///   Result: one node, fully embedded, with group metadata
    pub async fn find_and_tag_node(
        &self,
        owner_id:       Uuid,
        source_text:    &str,
        group_id:       &str,
        contributed_by: &str,
        status:         &str,
    ) -> anyhow::Result<String> {
        // Fetch all owner nodes to find the one ingestion just created
        let q = r#"query {
            nodes(zone: 0) {
                id sourceText domain kind
                isEmbedded embeddingDimensions
                metadata { key value }
            }
        }"#;

        let body = self.post(owner_id, q, json!({})).await?;

        let nodes = body["data"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        // Find embedded node matching source_text that has no group_id yet
        let target = nodes.iter().find(|n| {
            let text_match = n["sourceText"].as_str()
                .map(|t| t.trim() == source_text.trim())
                .unwrap_or(false);

            let is_embedded = n["isEmbedded"].as_bool().unwrap_or(false);

            let has_group = n["metadata"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .any(|m| m["key"].as_str() == Some("group_id"));

            text_match && is_embedded && !has_group
        });

        let (node_id, existing_meta, source_uri) = match target {
            Some(n) => {
                let id  = n["id"].as_str().unwrap_or("").to_string();
                let uri = n["sourceText"].as_str().unwrap_or("").to_string();
                let meta: Vec<Value> = n["metadata"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                (id, meta, uri)
            }
            None => {
                // Fallback — create a new node if ingestion node not found yet
                tracing::warn!(
                    "find_and_tag_node: no embedded node found for text, \
                     creating collab node without embeddings"
                );
                return self.upsert_group_node(
                    owner_id,
                    source_text,
                    &format!("collab://{group_id}/{}", Uuid::new_v4()),
                    group_id,
                    contributed_by,
                    status,
                    0,
                ).await;
            }
        };

        // Upsert the same node with group metadata added
        // Keeping all existing metadata (chunk_index, owner_id, token_count)
        let mut metadata = existing_meta;
        metadata.push(json!({ "key": "group_id",       "value": group_id }));
        metadata.push(json!({ "key": "contributed_by", "value": contributed_by }));
        metadata.push(json!({ "key": "status",         "value": status }));

        let q2 = r#"mutation($input: GqlNodeInput!) {
            upsertNode(input: $input) { id }
        }"#;

        self.post(owner_id, q2, json!({
            "input": {
                "id":         node_id,
                "domain":     "CUSTOM",
                "sourceUri":  source_uri,
                "sourceText": source_text,
                "kind":       "ARTIFCAT",
                "zone":       0,
                "metadata":   metadata,
            }
        })).await?;

        tracing::info!(
            node_id  = %node_id,
            group_id = %group_id,
            status   = %status,
            "node tagged for group"
        );

        Ok(node_id)
    }

    /// Upsert a node tagged with group_id in metadata.
    /// Used as fallback when ingestion node not found.
    pub async fn upsert_group_node(
        &self,
        owner_id:       Uuid,
        source_text:    &str,
        source_uri:     &str,
        group_id:       &str,
        contributed_by: &str,
        status:         &str,
        zone:           i32,
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
        // Fetch all nodes to find the one with this id
        let q = r#"query {
            nodes(zone: 0) {
                id sourceText domain kind
                metadata { key value }
            }
        }"#;

        let body = self.post(owner_id, q, json!({})).await?;
        let nodes = body["data"]["nodes"].as_array().cloned().unwrap_or_default();

        let target = nodes.iter().find(|n| n["id"].as_str() == Some(node_id));
        let (source_text, source_uri, existing_meta) = match target {
            Some(n) => (
                n["sourceText"].as_str().unwrap_or("").to_string(),
                String::new(),
                n["metadata"].as_array().cloned().unwrap_or_default(),
            ),
            None => return Ok(()),
        };

        let mut metadata: Vec<Value> = existing_meta.into_iter()
            .filter(|m| m["key"].as_str() != Some("status"))
            .collect();
        metadata.push(json!({ "key": "status", "value": status }));

        let q2 = r#"mutation($input: GqlNodeInput!) {
            upsertNode(input: $input) { id }
        }"#;

        self.post(owner_id, q2, json!({
            "input": {
                "id":         node_id,
                "domain":     "CUSTOM",
                "sourceUri":  source_uri,
                "sourceText": source_text,
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
                node {
                    id sourceText domain kind
                    metadata { key value }
                }
            }
        }"#;

        let body = self.post(owner_id, q, json!({
            "query":  query,
            "config": {
                "similarityTopK": (top_k * 10) as i32,
                "maxZone":        0,
            }
        })).await?;

        let results = body["data"]["search"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut matched: Vec<GraphNode> = results
            .into_iter()
            .filter_map(|r| {
                let score = r["score"].as_f64().unwrap_or(0.0) as f32;
                let n     = &r["node"];

                let meta: Vec<(String, String)> = n["metadata"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|m| Some((
                        m["key"].as_str()?.to_string(),
                        m["value"].as_str()?.to_string(),
                    )))
                    .collect();

                let node_group  = meta.iter()
                    .find(|(k, _)| k == "group_id")
                    .map(|(_, v)| v.as_str());
                let node_status = meta.iter()
                    .find(|(k, _)| k == "status")
                    .map(|(_, v)| v.as_str());

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
            })
            .collect();

        matched.sort_unstable_by(|a, b| {
            b.score.partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matched.truncate(top_k);

        Ok(matched)
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
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|n| Some(GraphNode {
                id:          n["id"].as_str()?.to_string(),
                source_text: n["sourceText"].as_str()?.to_string(),
                source_uri:  String::new(),
                domain:      n["domain"].as_str()?.to_string(),
                kind:        n["kind"].as_str()?.to_string(),
                metadata:    n["metadata"].as_array()
                    .cloned()
                    .unwrap_or_default()
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
        let q = r#"query {
            nodes(zone: 0) {
                id sourceText domain kind metadata { key value }
            }
        }"#;

        let body = self.post(owner_id, q, json!({})).await?;
        let nodes = body["data"]["nodes"].as_array().cloned().unwrap_or_default();

        Ok(nodes.into_iter()
            .find(|n| n["id"].as_str() == Some(node_id))
            .map(|n| GraphNode {
                id:          n["id"].as_str().unwrap_or("").to_string(),
                source_text: n["sourceText"].as_str().unwrap_or("").to_string(),
                source_uri:  String::new(),
                domain:      n["domain"].as_str().unwrap_or("").to_string(),
                kind:        n["kind"].as_str().unwrap_or("").to_string(),
                metadata:    n["metadata"].as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|m| Some((
                        m["key"].as_str()?.to_string(),
                        m["value"].as_str()?.to_string(),
                    )))
                    .collect(),
                score: 0.0,
            }))
    }

    // ── Internal ──────────────────────────────────────────────────────────────

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
            .context("failed to reach fluvio-graph")?;

        let body: Value = resp.json().await
            .context("failed to parse fluvio-graph response")?;

        if let Some(errors) = body.get("errors") {
            anyhow::bail!("fluvio-graph error: {errors}");
        }

        Ok(body)
    }
}