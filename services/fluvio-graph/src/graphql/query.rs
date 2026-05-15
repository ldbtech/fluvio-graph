//! Query resolvers for fluvio-graph GraphQL subgraph.
//!
//! Auth: user_id is injected by Apollo Router as `x-user-id` header.
//! Resolvers trust this header — JWT validation happens at the gateway only.

use async_graphql::*;
use uuid::Uuid;

use crate::server::AppState;
use crate::query_context::{QueryContext, QueryConfig, QueryRoute};
use crate::graphql::types::*;
use fluvio_types::{NodeId, GraphQuery};

pub struct QueryRoot;

#[Object]
impl QueryRoot {

    // ── Single node ───────────────────────────────────────────────────────────

    /// Fetch a single node by ID.
    async fn node(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<Option<GqlNode>> {
        let state   = ctx.data::<AppState>()?;
        let node_id = parse_node_id(&id)?;

        let row = state.surreal.get_node(&node_id).await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(row.map(|r| GqlNode::from(r.to_node())))
    }

    // ── User nodes ────────────────────────────────────────────────────────────

    /// Fetch all nodes for the authenticated user, optionally filtered by domain.
    async fn nodes(
        &self,
        ctx:    &Context<'_>,
        domain: Option<String>,
        zone:   Option<i32>,
    ) -> Result<Vec<GqlNode>> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let zone    = zone.unwrap_or(0) as i16;

        let rows = state.surreal
            .get_user_nodes(user_id, domain.as_deref(), zone)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(rows.into_iter().map(|r| GqlNode::from(r.to_node())).collect())
    }

    // ── Semantic search ───────────────────────────────────────────────────────

    /// Semantic similarity search — returns top-K nodes closest to the query.
    /// Goes directly to SurrealDB — no in-memory subgraph needed.
    async fn search(
        &self,
        ctx:    &Context<'_>,
        query:  String,
        config: Option<GqlQueryConfig>,
    ) -> Result<Vec<GqlScoredNode>> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let config  = config.map(QueryConfig::from).unwrap_or_default();

        // Embed the query
        let embedding = {
            let mut embedder = state.embedder.write().await;
            embedder.embed(&query)
                .map_err(|e| Error::new(format!("embed failed: {e:?}")))?
        };

        // Similarity search directly in SurrealDB
        let rows = state.surreal
            .similarity_search_nodes(
                user_id,
                &embedding,
                config.similarity_top_k,
                config.max_zone,
            )
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        // Score each result by cosine similarity with the query embedding
        let query_norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        let results = rows.into_iter().map(|row| {
            let node = row.to_node();
            let dot: f32 = embedding.iter()
                .zip(&node.embeddings)
                .map(|(a, b)| a * b)
                .sum();
            let node_norm: f32 = node.embeddings.iter()
                .map(|x| x * x).sum::<f32>().sqrt();
            let score = if node_norm == 0.0 || query_norm == 0.0 {
                0.0f64
            } else {
                (dot / (query_norm * node_norm)) as f64
            };
            GqlScoredNode { node: GqlNode::from(node), score }
        }).collect();

        Ok(results)
    }

    // ── Network search ────────────────────────────────────────────────────────

    /// Cross-user semantic search — "who in my network knows about X?"
    /// Zone 1 required — searches across connected users' nodes.
    async fn network_search(
        &self,
        ctx:      &Context<'_>,
        query:    String,
        user_ids: Vec<String>,
        top_k:    Option<i32>,
    ) -> Result<Vec<GqlScoredNode>> {
        let state   = ctx.data::<AppState>()?;
        let _caller = extract_user_id(ctx)?;

        let uuids: Vec<Uuid> = user_ids.iter()
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect();

        let embedding = {
            let mut embedder = state.embedder.write().await;
            embedder.embed(&query)
                .map_err(|e| Error::new(format!("embed failed: {e:?}")))?
        };

        let results = state.surreal
            .network_similarity_search(&uuids, &embedding, top_k.unwrap_or(20) as usize, 1)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        // For network search we return scored nodes with owner info in metadata
        let nodes = results.into_iter().map(|(owner_id, node_id_str, score)| {
            let mut metadata = vec![
                GqlMetadataEntry { key: "owner_id".into(), value: owner_id },
                GqlMetadataEntry { key: "node_id".into(),  value: node_id_str },
            ];
            GqlScoredNode {
                node: GqlNode {
                    id:          String::new(),
                    domain:      GqlDomain::Custom,
                    source_uri:  String::new(),
                    source_text: String::new(),
                    kind:        GqlNodeKind::Artifcat,
                    metadata,
                    embeddings:  vec![],
                },
                score: score as f64,
            }
        }).collect();

        Ok(nodes)
    }

    // ── BFS neighbors ─────────────────────────────────────────────────────────

    /// BFS expansion — returns all nodes within `depth` hops of the given node.
    /// Served directly from SurrealDB for depth <= 3.
    async fn neighbors(
        &self,
        ctx:   &Context<'_>,
        id:    String,
        depth: Option<i32>,
    ) -> Result<Vec<GqlNode>> {
        let state   = ctx.data::<AppState>()?;
        let _user   = extract_user_id(ctx)?;
        let node_id = parse_node_id(&id)?;
        let depth   = depth.unwrap_or(2) as usize;

        let rows = state.surreal.bfs(&node_id, depth).await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(rows.into_iter().map(|r| GqlNode::from(r.to_node())).collect())
    }

    // ── Shortest path ─────────────────────────────────────────────────────────

    /// Dual-weight Dijkstra shortest path between two nodes.
    /// Builds a request-scoped QueryContext — dropped after this resolver returns.
    async fn shortest_path(
        &self,
        ctx:  &Context<'_>,
        from: String,
        to:   String,
    ) -> Result<GqlPath> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let from_id = parse_node_id(&from)?;
        let to_id   = parse_node_id(&to)?;

        // Build request-scoped subgraph containing both nodes + their neighborhoods
        let config = QueryConfig::narrow();
        let qctx   = QueryContext::from_node_ids(
            user_id,
            &[from_id, to_id],
            &config,
            &state.surreal,
        ).await.map_err(|e| Error::new(e.to_string()))?;

        // Run Dijkstra on the tiny in-memory subgraph
        let result = qctx.query(GraphQuery::ShortestPath { from: from_id, to: to_id });

        match result {
            fluvio_types::GraphResult::Path(Some(path)) => {
                let nodes = path.into_iter()
                    .filter_map(|id| qctx.subgraph.nodes.get(&id).cloned())
                    .map(GqlNode::from)
                    .collect();
                Ok(GqlPath { nodes, found: true })
            }
            _ => Ok(GqlPath { nodes: vec![], found: false }),
        }
        // qctx dropped here — RAM freed
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract verified user_id from the x-user-id header injected by Apollo Router.
pub fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing or invalid — request must go through the gateway"))
}

fn parse_node_id(s: &str) -> Result<NodeId> {
    Uuid::parse_str(s)
        .map(NodeId)
        .map_err(|_| Error::new(format!("Invalid node ID: {s}")))
}