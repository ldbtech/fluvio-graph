//! QueryContext — request-scoped subgraph.
//!
//! - NO global in-memory graph
//! - NO per-user session cache
//! - Each query fetches ONLY the relevant subgraph from SurrealDB
//! - The subgraph lives in RAM for the duration of one request, then drops
//!
//! At 1B users with 10K concurrent requests and 500 nodes per context:
//!   10,000 × 500 × ~2KB = ~10GB RAM — fixed ceiling, independent of user count.

use uuid::Uuid;
use std::collections::HashSet;
use std::sync::Arc;

use fluvio_types::{DomainGraph, GraphId, Domain, GraphQuery, GraphResult};

use crate::graph::FluvioGraph;   // ← brings .query() into scope for DomainGraph
use crate::storage::surreal::SurrealStorage;
use crate::embeddings::EmbeddingContext;

// ── QueryConfig ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub similarity_top_k:   usize,
    pub expansion_depth:    usize,
    pub max_subgraph_nodes: usize,
    pub max_zone:           i16,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            similarity_top_k:   50,
            expansion_depth:    2,
            max_subgraph_nodes: 500,
            max_zone:           0,
        }
    }
}

impl QueryConfig {
    pub fn network() -> Self { Self { max_zone: 1, ..Self::default() } }
    pub fn narrow()  -> Self { Self { similarity_top_k: 20, max_subgraph_nodes: 150, ..Self::default() } }
    pub fn deep()    -> Self { Self { similarity_top_k: 100, expansion_depth: 3, max_subgraph_nodes: 1000, ..Self::default() } }
}

// ── QueryContext ──────────────────────────────────────────────────────────────

pub struct QueryContext {
    pub subgraph:      DomainGraph,
    pub user_id:       Uuid,
    pub node_count:    usize,
    pub fetched_count: usize,
}

impl QueryContext {
    /// Build from natural-language query string.
    pub async fn from_text(
        user_id:  Uuid,
        query:    &str,
        config:   &QueryConfig,
        surreal:  &Arc<SurrealStorage>,
        embedder: &mut EmbeddingContext,
        workspace_id: Option<&str>,
    ) -> anyhow::Result<Self> {
        let embedding = embedder.embed(query)
            .map_err(|e| anyhow::anyhow!("embed failed: {e:?}"))?;
        Self::from_embedding(user_id, &embedding, config, surreal, workspace_id).await
    }

    /// Build from a pre-computed embedding vector.
    pub async fn from_embedding(
        user_id:   Uuid,
        embedding: &[f32],
        config:    &QueryConfig,
        surreal:   &Arc<SurrealStorage>,
        workspace_id: Option<&str>,
    ) -> anyhow::Result<Self> {
        // Step 1: similarity search → seed nodes
        let seeds = surreal.similarity_search_nodes(
            user_id,
            embedding,
            config.similarity_top_k,
            config.max_zone,
            workspace_id,
        ).await?;

        let fetched_count = seeds.len();
        let mut all_rows  = seeds;

        // Step 2: BFS expansion around each seed
        let seed_ids: Vec<fluvio_types::NodeId> = all_rows.iter()
            .map(|r| {
                let id_str = format!("{:?}", r.id);
                fluvio_types::NodeId(
                    uuid::Uuid::parse_str(
                        id_str.trim_start_matches("nodes:")
                              .trim_matches('`')
                              .trim()
                    ).unwrap_or_default()
                )
            })
            .collect();

        let mut seen: HashSet<String> = all_rows.iter()
            .map(|r| format!("{:?}", r.id))
            .collect();

        for seed_id in &seed_ids {
            if all_rows.len() >= config.max_subgraph_nodes { break; }
            let neighbors = surreal.bfs(seed_id, config.expansion_depth).await?;
            for row in neighbors {
                let key = format!("{:?}", row.id);
                if seen.insert(key) {
                    all_rows.push(row);
                    if all_rows.len() >= config.max_subgraph_nodes { break; }
                }
            }
        }

        // Step 3: cap + build subgraph
        all_rows.truncate(config.max_subgraph_nodes);
        let node_count = all_rows.len();

        let mut subgraph = DomainGraph::new(
            GraphId::new(&format!("ctx:{user_id}")),
            Domain::Custom("query_context".into()),
        );
        for row in all_rows {
            subgraph.insert_node(row.to_node());
        }

        tracing::debug!(user_id = %user_id, node_count, fetched_count, "QueryContext built");

        Ok(Self { subgraph, user_id, node_count, fetched_count })
    }

    /// Build from a known set of node IDs.
    pub async fn from_node_ids(
        user_id:  Uuid,
        node_ids: &[fluvio_types::NodeId],
        config:   &QueryConfig,
        surreal:  &Arc<SurrealStorage>,
    ) -> anyhow::Result<Self> {
        let mut subgraph = DomainGraph::new(
            GraphId::new(&format!("ctx:{user_id}")),
            Domain::Custom("query_context".into()),
        );

        let mut fetched = 0usize;
        for id in node_ids.iter().take(config.max_subgraph_nodes) {
            if let Some(row) = surreal.get_node(id).await? {
                subgraph.insert_node(row.to_node());
                fetched += 1;
            }
        }

        Ok(Self {
            node_count:    subgraph.node_count(),
            fetched_count: fetched,
            subgraph,
            user_id,
        })
    }

    /// Run a graph algorithm against this request's subgraph.
    /// Only use for algorithms SurrealQL cannot express natively.
    pub fn query(&self, q: GraphQuery) -> GraphResult {
        self.subgraph.query(q)
    }

    pub fn node_count(&self) -> usize { self.node_count }
    pub fn is_empty(&self)  -> bool   { self.node_count == 0 }
}

// ── QueryRoute ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum QueryRoute {
    DirectDb,
    SubgraphRequired,
}

impl QueryRoute {
    pub fn for_query(q: &GraphQuery) -> Self {
        match q {
            GraphQuery::SimilarTo { .. }                        => Self::DirectDb,
            GraphQuery::RefsForDomain(_)                        => Self::DirectDb,
            GraphQuery::Bfs { .. }                              => Self::DirectDb,
            GraphQuery::Neighbors { depth, .. } if *depth <= 3 => Self::DirectDb,
            GraphQuery::ShortestPath { .. }                     => Self::SubgraphRequired,
            GraphQuery::Filter(_)                               => Self::SubgraphRequired,
            GraphQuery::Neighbors { .. }                        => Self::SubgraphRequired,
        }
    }

    pub fn needs_subgraph(&self) -> bool { self == &Self::SubgraphRequired }
}