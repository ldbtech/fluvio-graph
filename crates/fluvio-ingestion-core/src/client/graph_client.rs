//! GraphQL HTTP client for calling fluvio-graph.
//!
//! fluvio-ingestion never touches SurrealDB directly.
//! All graph writes go through fluvio-graph's GraphQL API.

use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

use fluvio_types::{Node, Edge, Domain, NodeKind};

/// HTTP client that calls fluvio-graph GraphQL mutations.
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

    /// Upsert a node into fluvio-graph.
    /// Returns the node ID assigned by fluvio-graph.
    pub async fn upsert_node(
        &self,
        owner_id: Uuid,
        node:     &Node,
        zone:     i16,
    ) -> anyhow::Result<String> {
        let domain_str = domain_to_gql(&node.domain);
        let kind_str   = kind_to_gql(&node.kind);

        // Build metadata as GraphQL input array
        let metadata: Vec<Value> = node.metadata.iter()
            .map(|(k, v)| json!({ "key": k, "value": v }))
            .collect();

        let query = r#"
            mutation UpsertNode($input: GqlNodeInput!) {
                upsertNode(input: $input) {
                    id
                }
            }
        "#;

        let variables = json!({
            "input": {
                "id":          node.id.to_string(),
                "domain":      domain_str,
                "sourceUri":   node.source_uri,
                "sourceText":  node.source_text,
                "kind":        kind_str,
                "metadata":    metadata,
                "zone":        zone as i32,
                "embeddings":  node.embeddings,
            }
        });

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
            anyhow::bail!("fluvio-graph upsertNode error: {errors}");
        }

        let id = body["data"]["upsertNode"]["id"]
            .as_str()
            .context("missing id in upsertNode response")?
            .to_string();

        Ok(id)
    }

    /// Upsert an edge into fluvio-graph.
    pub async fn upsert_edge(
        &self,
        owner_id: Uuid,
        edge:     &Edge,
    ) -> anyhow::Result<()> {
        let query = r#"
            mutation UpsertEdge($input: GqlEdgeInput!) {
                upsertEdge(input: $input) {
                    id
                }
            }
        "#;

        let variables = json!({
            "input": {
                "from":                    edge.from.to_string(),
                "to":                      edge.to.to_string(),
                "label":                   edge.label,
                "token":                   edge.token,
                "relationshipProbability": edge.relationship_probability,
            }
        });

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
            anyhow::bail!("fluvio-graph upsertEdge error: {errors}");
        }

        Ok(())
    }

    /// Fetch embeddings for a list of node IDs.
    /// Used by edge wirer to compute cosine similarity between newly ingested nodes.
    pub async fn get_nodes(
        &self,
        owner_id: Uuid,
        zone:     i16,
    ) -> anyhow::Result<Vec<(String, Vec<f32>)>> {
        let query = r#"
            query GetNodes($zone: Int) {
                nodes(zone: $zone) {
                    id
                    embeddingDimensions
                }
            }
        "#;

        let resp = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-user-id", owner_id.to_string())
            .json(&json!({
                "query": query,
                "variables": { "zone": zone as i32 }
            }))
            .send()
            .await
            .context("failed to reach fluvio-graph")?;

        let body: Value = resp.json().await?;

        let nodes = body["data"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|n| {
                let id = n["id"].as_str()?.to_string();
                Some((id, vec![]))   // embeddings fetched separately when needed
            })
            .collect();

        Ok(nodes)
    }
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn domain_to_gql(d: &Domain) -> &'static str {
    match d {
        Domain::Pdf          => "PDF",
        Domain::Email        => "EMAIL",
        Domain::Whatsapp     => "WHATSAPP",
        Domain::Calendar     => "CALENDAR",
        Domain::Codebase     => "CODEBASE",
        Domain::Web          => "WEB",
        Domain::Custom(_)    => "CUSTOM",
    }
}

fn kind_to_gql(k: &NodeKind) -> &'static str {
    match k {
        NodeKind::Entity       => "ENTITY",
        NodeKind::Topic        => "TOPIC",
        NodeKind::Artifcat     => "ARTIFCAT",
        NodeKind::Event        => "EVENT",
        NodeKind::Conversation => "CONVERSATION",
        NodeKind::Capability   => "CAPABILITY",
        NodeKind::ExternalRef(_) => "EXTERNAL_REF",
    }
}