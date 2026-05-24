//! Node types for the Fluvio graph.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::graph::ids::{GraphId, NodeId};
use crate::graph::enums::{Domain, NodeKind};

// ── ExternalRef ───────────────────────────────────────────────────────────────

/// A cross-domain pointer that lives in the MetaGraph.
///
/// When a node in graph A refers to a node in graph B, rather than
/// duplicating the target node, a `NodeKind::ExternalRef(ExternalRef)`
/// is created in graph A pointing at the target by (graph_id, node_id, domain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRef {
    pub graph_id: GraphId,
    pub node_id:  NodeId,
    pub domain:   Domain,
}

// ── Node ──────────────────────────────────────────────────────────────────────

/// A vertex in a `DomainGraph`.
///
/// A node is the atomic unit of knowledge in Fluvio. Every ingested
/// document chunk, email thread, code entity, or connector data point
/// becomes a `Node`. The `embeddings` field enables semantic search.
///
/// ## Field notes
/// - `source_uri` — opaque locator: file path, message-id, Spotify URI, GitHub URL, …
/// - `source_text` — the canonical extractable text used for embedding + LLM context
/// - `embeddings` — BGE-small vector (384 dimensions); empty until the embedder runs
/// - `metadata` — arbitrary key-value bag; never used for graph traversal logic
/// - `kind` — semantic classification; see [`NodeKind`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id:          NodeId,
    pub domain:      Domain,
    pub source_uri:  String,
    pub source_text: String,
    pub embeddings:  Vec<f32>,
    pub metadata:    HashMap<String, String>,
    pub kind:        NodeKind,
    pub zone:        i16,
}

impl Node {
    /// Convenience constructor — embeddings are left empty until
    /// `fluvio-embed` runs the BGE-small model over `source_text`.
    pub fn new(
        id:          NodeId,
        domain:      Domain,
        source_uri:  impl Into<String>,
        source_text: impl Into<String>,
        kind:        NodeKind,
    ) -> Self {
        Self {
            id,
            domain,
            source_uri:  source_uri.into(),
            source_text: source_text.into(),
            embeddings:  Vec::new(),
            metadata:    HashMap::new(),
            kind,
            zone:        0,
        }
    }

    /// Returns `true` if the embedding vector has been populated.
    pub fn is_embedded(&self) -> bool {
        !self.embeddings.is_empty()
    }
}