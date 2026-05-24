//! GraphQL client for calling fluvio-graph from fluvio-twin.
//!
//! fluvio-twin never touches SurrealDB directly.
//! All graph reads go through fluvio-graph's GraphQL API.

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

// ── Types returned from fluvio-graph ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id:          String,
    pub source_text: String,
    pub source_uri:  String,
    pub domain:      String,
    pub kind:        String,
    pub metadata:    Vec<(String, String)>,
    pub score:       f32,
    pub zone:        i32,
}

#[derive(Clone)]
pub struct GraphClient {
    pub endpoint: String,
    pub client:   Client,
}

impl GraphClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client:   Client::new(),
        }
    }

    /// Semantic similarity search — top-K nodes closest to the query.
    pub async fn search(
        &self,
        owner_id: Uuid,
        query:    &str,
        top_k:    usize,
        zone:     i16,
    ) -> anyhow::Result<Vec<GraphNode>> {
        let gql = r#"
            query Search($query: String!, $config: GqlQueryConfig) {
                search(query: $query, config: $config) {
                    score
                    node {
                        id
                        sourceText
                        sourceUri: sourceUri
                        domain
                        kind
                        zone
                        metadata { key value }
                    }
                }
            }
        "#;

        let variables = json!({
            "query": query,
            "config": {
                "similarityTopK": top_k as i32,
                "maxZone":        zone as i32,
            }
        });

        let body: Value = self.post(owner_id, gql, variables).await?;

        let results = body["data"]["search"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(results.into_iter().map(|r| {
            let score = r["score"].as_f64().unwrap_or(0.0) as f32;
            let n     = &r["node"];
            GraphNode {
                id:          n["id"].as_str().unwrap_or("").to_string(),
                source_text: n["sourceText"].as_str().unwrap_or("").to_string(),
                source_uri:  n["sourceUri"].as_str().unwrap_or("").to_string(),
                domain:      n["domain"].as_str().unwrap_or("").to_string(),
                kind:        n["kind"].as_str().unwrap_or("").to_string(),
                metadata:    n["metadata"].as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|m| {
                        let k = m["key"].as_str()?.to_string();
                        let v = m["value"].as_str()?.to_string();
                        Some((k, v))
                    })
                    .collect(),
                score,
                zone:        n["zone"].as_i64().unwrap_or(0) as i32,
            }
        }).collect())
    }

    /// BFS expansion — neighbors within `depth` hops of a node.
    pub async fn neighbors(
        &self,
        owner_id: Uuid,
        node_id:  &str,
        depth:    usize,
    ) -> anyhow::Result<Vec<GraphNode>> {
        let gql = r#"
            query Neighbors($id: String!, $depth: Int) {
                neighbors(id: $id, depth: $depth) {
                    id
                    sourceText
                    domain
                    kind
                    zone
                    metadata { key value }
                }
            }
        "#;

        let variables = json!({
            "id":    node_id,
            "depth": depth as i32,
        });

        let body: Value = self.post(owner_id, gql, variables).await?;

        let nodes = body["data"]["neighbors"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(nodes.into_iter().map(|n| GraphNode {
            id:          n["id"].as_str().unwrap_or("").to_string(),
            source_text: n["sourceText"].as_str().unwrap_or("").to_string(),
            source_uri:  String::new(),
            domain:      n["domain"].as_str().unwrap_or("").to_string(),
            kind:        n["kind"].as_str().unwrap_or("").to_string(),
            metadata:    n["metadata"].as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|m| {
                    let k = m["key"].as_str()?.to_string();
                    let v = m["value"].as_str()?.to_string();
                    Some((k, v))
                })
                .collect(),
            score: 0.0,
            zone:        n["zone"].as_i64().unwrap_or(0) as i32,
        }).collect())
    }

    /// Fetch all nodes for a user — for document listing.
    pub async fn nodes(
        &self,
        owner_id: Uuid,
        zone:     i16,
    ) -> anyhow::Result<Vec<GraphNode>> {
        let gql = r#"
            query Nodes($zone: Int) {
                nodes(zone: $zone) {
                    id
                    sourceText
                    sourceUri: sourceUri
                    domain
                    kind
                    isEmbedded
                    zone
                    metadata { key value }
                }
            }
        "#;

        let variables = json!({ "zone": zone as i32 });
        let body: Value = self.post(owner_id, gql, variables).await?;

        let nodes = body["data"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(nodes.into_iter().map(|n| GraphNode {
            id:          n["id"].as_str().unwrap_or("").to_string(),
            source_text: n["sourceText"].as_str().unwrap_or("").to_string(),
            source_uri:  n["sourceUri"].as_str().unwrap_or("").to_string(),
            domain:      n["domain"].as_str().unwrap_or("").to_string(),
            kind:        n["kind"].as_str().unwrap_or("").to_string(),
            metadata:    n["metadata"].as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|m| {
                    let k = m["key"].as_str()?.to_string();
                    let v = m["value"].as_str()?.to_string();
                    Some((k, v))
                })
                .collect(),
            score: 0.0,
            zone:        n["zone"].as_i64().unwrap_or(0) as i32,
        }).collect())
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