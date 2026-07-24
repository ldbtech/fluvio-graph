//! Semantic edge wiring.
//!
//! After ingesting a batch of nodes, wire semantic edges between nodes
//! whose embeddings are cosine-similar above the threshold.
//!
//! This is called AFTER all nodes are written to fluvio-graph so that
//! the Dijkstra shortest-path traversal has meaningful edges to follow.
//!
//! Threshold: 0.35 (from original monolith design)
//! Edge label: "semantic_neighbor"

use std::sync::Arc;
use uuid::Uuid;
use fluvio_types::{Edge, EdgeId, NodeId};
use crate::client::GraphClient;

pub const SEMANTIC_EDGE_THRESHOLD: f32 = 0.35;
pub const MAX_NEIGHBORS_PER_NODE:  usize = 6;

/// Wire semantic edges between a set of newly ingested nodes.
///
/// For each node pair (i, j) where cosine_sim(i, j) >= threshold,
/// creates a bidirectional "semantic_neighbor" edge.
///
/// Token cost is derived from similarity: cost = (1 - sim) * 10_000
/// This means high-similarity edges are cheaper to traverse (Dijkstra prefers them).
pub async fn wire_edges(
    owner_id:   Uuid,
    nodes:      &[(NodeId, Vec<f32>)],   // (node_id, embedding)
    client:     &Arc<GraphClient>,
    threshold:  f32,
) -> anyhow::Result<usize> {
    let mut edges_wired = 0usize;

    for (i, (id_a, emb_a)) in nodes.iter().enumerate() {
        // Score all other nodes against this one
        let mut scores: Vec<(usize, f32)> = nodes.iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, (_, emb_b))| (j, cosine_sim(emb_a, emb_b)))
            .filter(|(_, sim)| *sim >= threshold)
            .collect();

        // Sort descending, take top N
        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.truncate(MAX_NEIGHBORS_PER_NODE);

        for (j, sim) in scores {
            let id_b      = nodes[j].0;
            let token_cost = ((1.0 - sim) * 10_000.0) as i32;

            let edge = Edge {
                id:                       EdgeId::new(),
                from:                     *id_a,
                to:                       id_b,
                token:                    token_cost,
                relationship_probability: sim as f64,
                label:                    "semantic_neighbor".to_string(),
                metadata:                 std::collections::HashMap::new(),
            };

            match client.upsert_edge(owner_id, &edge).await {
                Ok(_)  => edges_wired += 1,
                Err(e) => tracing::warn!("failed to wire edge {id_a}→{id_b}: {e}"),
            }
        }
    }

    tracing::info!(edges_wired, "semantic edges wired");
    Ok(edges_wired)
}

/// Cosine similarity between two embedding vectors.
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32  = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na:  f32  = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb:  f32  = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}