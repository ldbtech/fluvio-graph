//! Mutation resolvers for fluvio-graph GraphQL subgraph.

use async_graphql::*;
use std::collections::HashMap;
use uuid::Uuid;

use fluvio_types::{Node, NodeId, Edge, EdgeId, NodeKind, Domain};

use crate::server::AppState;
use crate::graphql::types::*;
use crate::graphql::query::extract_user_id;

pub struct MutationRoot;

#[Object(name = "Mutation")]
impl MutationRoot {

    // ── Upsert node ───────────────────────────────────────────────────────────

    async fn upsert_node(
        &self,
        ctx:   &Context<'_>,
        input: GqlNodeInput,
    ) -> Result<GqlNode> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
    
        let node_id = match &input.id {
            Some(s) => Uuid::parse_str(s)
                .map(NodeId)
                .map_err(|_| Error::new(format!("Invalid node ID: {s}")))?,
            None => NodeId::random(),
        };
    
        let metadata: HashMap<String, String> = input.metadata
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.key, m.value))
            .collect();
    
        // If updating an existing node and no embeddings provided,
        // preserve the existing embeddings from SurrealDB.
        // This prevents find_and_tag_node from wiping embeddings when
        // adding group metadata to an already-embedded node.
        let embeddings = match input.embeddings {
            Some(ref e) if !e.is_empty() => e.clone(),
            _ if input.id.is_some() => {
                state.surreal.get_node(&node_id).await
                    .ok()
                    .flatten()
                    .map(|r| r.embeddings)
                    .unwrap_or_default()
            }
            _ => vec![],
        };
    
        let node = Node {
            id:          node_id,
            domain:      Domain::from(input.domain),
            source_uri:  input.source_uri,
            source_text: input.source_text.clone(),
            embeddings,
            metadata,
            kind:        NodeKind::from(input.kind),
        };
    
        let zone = input.zone.unwrap_or(0) as i16;
    
        state.surreal.upsert_node(user_id, &node, zone).await
            .map_err(|e| Error::new(e.to_string()))?;
    
        tracing::info!(user_id = %user_id, node_id = %node_id, "node upserted");
    
        Ok(GqlNode {
            id:          node.id.to_string(),
            domain:      GqlDomain::from(&node.domain),
            source_uri:  node.source_uri,
            source_text: node.source_text,
            kind:        GqlNodeKind::from(&node.kind),
            metadata:    node.metadata.into_iter()
                .map(|(k, v)| GqlMetadataEntry { key: k, value: v })
                .collect(),
            embeddings:  node.embeddings,
        })
    }

    // ── Delete node ───────────────────────────────────────────────────────────

    async fn delete_node(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<bool> {
        let state   = ctx.data::<AppState>()?;
        let _user   = extract_user_id(ctx)?;

        let node_id = Uuid::parse_str(&id)
            .map(NodeId)
            .map_err(|_| Error::new(format!("Invalid node ID: {id}")))?;

        state.surreal.delete_node_records(&[node_id]).await
            .map_err(|e| Error::new(e.to_string()))?;

        tracing::info!(node_id = %node_id, "node deleted");
        Ok(true)
    }

    // ── Upsert edge ───────────────────────────────────────────────────────────

    async fn upsert_edge(
        &self,
        ctx:   &Context<'_>,
        input: GqlEdgeInput,
    ) -> Result<GqlEdge> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;

        let from = Uuid::parse_str(&input.from)
            .map(NodeId)
            .map_err(|_| Error::new("Invalid from node ID"))?;
        let to = Uuid::parse_str(&input.to)
            .map(NodeId)
            .map_err(|_| Error::new("Invalid to node ID"))?;

        let edge = Edge {
            id:                       EdgeId::new(),
            from,
            to,
            label:                    input.label.clone(),
            token:                    input.token.unwrap_or(1),
            relationship_probability: input.relationship_probability.unwrap_or(0.9),
            metadata:                 HashMap::new(),
        };

        state.surreal.upsert_edge(user_id, &edge).await
            .map_err(|e| Error::new(e.to_string()))?;

        tracing::info!(
            user_id = %user_id,
            from    = %from,
            to      = %to,
            label   = %input.label,
            "edge upserted"
        );

        Ok(GqlEdge {
            id:                       edge.id.to_string(),
            from:                     edge.from.to_string(),
            to:                       edge.to.to_string(),
            label:                    edge.label,
            token:                    edge.token,
            relationship_probability: edge.relationship_probability,
            metadata:                 vec![],
        })    }

    // ── Save graph ────────────────────────────────────────────────────────────

    async fn save_graph(
        &self,
        ctx:  &Context<'_>,
        zone: Option<i32>,
    ) -> Result<GqlSaveResult> {
        let _state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let zone    = zone.unwrap_or(0) as i16;

        tracing::info!(user_id = %user_id, zone = zone, "save_graph called");
        Ok(GqlSaveResult { nodes_saved: 0, edges_saved: 0 })
    }

    // ── Delete user graph ─────────────────────────────────────────────────────

    async fn delete_user_graph(&self, ctx: &Context<'_>) -> Result<bool> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;

        state.surreal.delete_user_graph(user_id).await
            .map_err(|e| Error::new(e.to_string()))?;

        tracing::info!(user_id = %user_id, "user graph deleted");
        Ok(true)
    }
}

// ── Result types ──────────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct GqlSaveResult {
    pub nodes_saved: i32,
    pub edges_saved: i32,
}