//! Edge types for the Fluvio graph.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::graph::ids::{EdgeId, NodeId};

// ── Edge ──────────────────────────────────────────────────────────────────────

/// A directed, weighted relationship between two nodes.
///
/// Fluvio uses a **dual-weight** edge model analogous to A* pathfinding:
///
/// ```text
/// traversal_cost = token + (1.0 - relationship_probability) * 1000
/// ```
///
/// - `token` — approximate LLM token cost of serialising this edge's context
/// - `relationship_probability` — confidence \[0.0, 1.0\] that the relationship
///   is semantically real. Derived from cosine similarity for structural edges,
///   or from LLM extraction confidence for semantic edges.
///
/// The dual-weight formulation means the shortest-path algorithm
/// simultaneously minimises token cost and maximises relationship confidence.
///
/// ## Label convention
/// Labels use snake_case verb phrases that read left-to-right:
/// `"authored_by"`, `"references"`, `"co_occurs_with"`, `"implements"`, …
///
/// Labels become SurrealDB relation table names — they must be alphanumeric + underscore.
/// The storage layer calls `sanitize_label()` before writing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id:   EdgeId,
    pub from: NodeId,
    pub to:   NodeId,

    /// Approximate LLM token cost of traversing / serialising this edge.
    pub token: i32,

    /// Confidence \[0.0, 1.0\] that this relationship is semantically real.
    /// Computed as cosine similarity for structural edges (≥ 0.35 threshold).
    pub relationship_probability: f64,

    /// Human-readable relationship label, e.g. `"authored_by"`, `"references"`.
    pub label: String,

    /// Arbitrary key-value metadata; not used in traversal logic.
    pub metadata: HashMap<String, String>,
}

impl Edge {
    pub fn new(
        from:                    NodeId,
        to:                      NodeId,
        label:                   impl Into<String>,
        token:                   i32,
        relationship_probability: f64,
    ) -> Self {
        Self {
            id:   EdgeId::new(),
            from,
            to,
            token,
            relationship_probability,
            label:    label.into(),
            metadata: HashMap::new(),
        }
    }
}