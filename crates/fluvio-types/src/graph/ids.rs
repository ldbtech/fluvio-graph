//! Identifier types for the Fluvio graph.
//!
//! All IDs are either UUID-backed or string-backed.
//! `NodeId` supports content-addressing via `from_content()` so two
//! independent ingestion pipelines producing the same canonical entity
//! will converge on the same node without a dedup pass.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── NodeId ────────────────────────────────────────────────────────────────────

/// Stable, globally unique identifier for a graph node.
///
/// Supports two construction modes:
/// - [`NodeId::random()`] — opaque UUID v4, for nodes with no stable canonical form
/// - [`NodeId::from_content()`] — deterministic from (source_type, text), enables
///   content-addressed deduplication across ingestion runs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Content-addressed: deterministic from source type + canonical text.
    ///
    /// Two sources both extracting "Peter Steinberger" as a Person entity
    /// will produce the **same** `NodeId` — no dedup query needed.
    ///
    /// Implementation: BLAKE3(source_type + "::" + text.trim().to_lowercase())
    /// → first 16 bytes interpreted as a UUID.
    pub fn from_content(source_type: &str, canonical_text: &str) -> Self {
        let input = format!("{}::{}", source_type, canonical_text.trim().to_lowercase());
        let hash  = blake3::hash(input.as_bytes());
        let bytes: [u8; 16] = hash.as_bytes()[..16].try_into().unwrap();
        Self(Uuid::from_bytes(bytes))
    }

    /// Random UUID v4 — use when the node has no stable canonical form.
    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── EdgeId ────────────────────────────────────────────────────────────────────

/// Stable identifier for a directed edge between two nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);

impl EdgeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── GraphId ───────────────────────────────────────────────────────────────────

/// Human-readable identifier for a `DomainGraph` instance.
///
/// Used as a namespace key in the `GraphRegistry` and as the SurrealDB
/// namespace discriminator when multiple graphs are loaded simultaneously.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphId(pub String);

impl GraphId {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for GraphId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}