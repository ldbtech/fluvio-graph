//! Graph-level enumerations.
//!
//! Moved verbatim from `src/graph/enums.rs` in the monolith.
//! The `NodeKind::ExternalRef` variant carries an `ExternalRef` value
//! defined in `graph::node` — both live in this crate so there is no
//! circular dependency.
//!
//! ## Serialization note
//! `NodeKind` and `Domain` are stored as strings in SurrealDB via
//! `format!("{:?}", value)`. Do NOT rename variants without a migration.
//! The typo `Artifcat` is intentionally preserved for backwards compatibility
//! with existing SurrealDB records.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::graph::ids::{EdgeId, NodeId};
use crate::graph::node::{ExternalRef, Node};

// ── GraphResult ───────────────────────────────────────────────────────────────

/// Return type for graph query operations.
#[derive(Debug, Clone)]
pub enum GraphResult {
    Nodes(Vec<Node>),
    Path(Option<Vec<NodeId>>),
    Scored(Vec<(NodeId, f32)>),
    Empty,
}

// ── GraphError ────────────────────────────────────────────────────────────────

/// Error type for graph operations.
#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Node {0:?} not found")]
    NodeNotFound(NodeId),

    #[error("Edge {0:?} not found")]
    EdgeNotFound(EdgeId),

    #[error("Embedding failed: {0}")]
    EmbeddingFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Storage error: {0}")]
    StorageError(String),
}

// ── GraphEvent ────────────────────────────────────────────────────────────────

/// Events broadcast on the `DomainGraph` tokio channel.
/// Used by in-memory subscribers (e.g. live sync to SurrealDB LIVE SELECT).
#[derive(Debug, Clone)]
pub enum GraphEvent {
    NodeInserted(NodeId),
    NodeUpdated(NodeId),
    NodeDeleted(NodeId),
    EdgeInserted(EdgeId),
}

// ── GraphQuery ────────────────────────────────────────────────────────────────

/// Typed query variants issued against a `DomainGraph`.
/// Each variant maps to one traversal method on `DomainGraph`.
#[derive(Debug, Clone)]
pub enum GraphQuery {
    /// All neighbours of a node up to depth N.
    Neighbors { root: NodeId, depth: usize },

    /// Weighted shortest path (dual-weight Dijkstra: token cost + uncertainty).
    ShortestPath { from: NodeId, to: NodeId },

    /// All nodes matching a predicate.
    Filter(NodePredicate),

    /// BFS visit order from root.
    Bfs { root: NodeId },

    /// Semantic nearest neighbours by cosine similarity.
    SimilarTo { embedding: Vec<f32>, top_k: usize },

    /// All ExternalRef nodes pointing to a given domain.
    RefsForDomain(Domain),
}

// ── NodeKind ──────────────────────────────────────────────────────────────────

/// What kind of semantic entity a node represents.
///
/// ⚠️  `Artifcat` is a preserved typo — do not fix it.
///     Existing SurrealDB records store the string `"Artifcat"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    /// A named real-world entity: person, organisation, location, product.
    Entity,
    /// A concept, idea, theme, or category.
    Topic,
    /// A document, file, image, video, or other artefact.
    Artifcat,
    /// A time-bounded occurrence: meeting, deadline, event.
    Event,
    /// A chat, thread, or discussion.
    Conversation,
    /// A synthesized or registered CSP capability — a general, reusable verb.
    /// `source_text` holds the spec (embedded for reuse-first search); the
    /// generated code + signature live in `metadata`.
    Capability,
    /// A pointer into another graph (used in the MetaGraph only).
    ExternalRef(ExternalRef),
}

// ── NodeKindFilter ────────────────────────────────────────────────────────────

/// Variant-only mirror of `NodeKind` used in predicate matching
/// so callers don't need to construct a full `ExternalRef` to filter by kind.
#[derive(Debug, Clone)]
pub enum NodeKindFilter {
    Entity,
    Topic,
    Artifcat,
    Event,
    Conversation,
    Capability,
    ExternalRef,
}

// ── NodePredicate ─────────────────────────────────────────────────────────────

/// Composable predicate tree for filtering nodes in a `DomainGraph`.
///
/// Predicates are evaluated in-memory by `DomainGraph::matches_predicate`.
/// For SurrealDB queries use the equivalent WHERE clause builders in
/// `services/fluvio-graph/src/storage/surreal.rs`.
#[derive(Debug, Clone)]
pub enum NodePredicate {
    ByDomain(Domain),
    ByKind(NodeKindFilter),
    ByMetadata { key: String, value: String },
    And(Box<NodePredicate>, Box<NodePredicate>),
    Or(Box<NodePredicate>, Box<NodePredicate>),
}

// ── Domain ────────────────────────────────────────────────────────────────────

/// The data domain a node or graph originates from.
///
/// `Custom(String)` accommodates domains added by connectors at runtime
/// (e.g. `Domain::Custom("yahoo_finance")`, `Domain::Custom("github")`).
///
/// ⚠️  Variant names are stored as `"{:?}"` strings in SurrealDB.
///     Do not rename existing variants without a migration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    Pdf,
    Email,
    Whatsapp,
    Calendar,
    Codebase,
    Web,
    /// Connector-defined domain, e.g. "yahoo_finance", "github", "agent_pdf"
    Custom(String),
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::Custom(s) => write!(f, "{}", s),
            other => write!(f, "{:?}", other),
        }
    }
}