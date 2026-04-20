//! connector.rs
//!
//! Core ingestion abstractions for Fluvio.
//!
//! Every source (PDF, Email, WhatsApp, Music, Codebase...) implements
//! `FluvioConnector` and produces `NormalizedChunk` — the universal
//! intermediate format the ingestion pipeline consumes.
//!
//! The pipeline never knows what the source was. It just sees chunks.
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
 
use crate::graph::enums::Domain;

// ── Errors ────────────────────────────────────────────────────────────────────
#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error("auth error: {0}")]
    Auth(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("not configured: {0}")]
    NotConfigured(String),
}

// ── NormalizedChunk ───────────────────────────────────────────────────────────
 
/// Universal intermediate format produced by every connector.
/// The ingestion pipeline embeds `text`, creates a Node from it,
/// and stores `metadata` on the node.
///
/// `source_uri` is always set so any node can be traced back to its origin:
///   PDF   → "pdf:///path/to/file.pdf#page=3"
///   Email → "gmail://message/abc123"
///   Music → "spotify://track/xyz"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedChunk {
    /// Embeddable text content — what gets turned into a vector.
    pub text: String,
 
    /// Arbitrary key-value metadata stored on the graph node.
    /// Common keys: page, sender, subject, timestamp, label, thread_id,
    ///              file, function, artist, album, language.
    pub metadata: HashMap<String, String>,
 
    /// Zero-based index of this chunk within its source document/thread/file.
    pub chunk_index: usize,
 
    /// Stable URI identifying the exact origin of this chunk.
    pub source_uri: String,
 
    /// The domain this chunk belongs to — determines which graph it enters.
    pub domain: Domain,
 
    /// Pre-defined edges to other chunks by source_uri.
    /// Set by structured connectors (email threads, code call graphs)
    /// that already know the relationships without needing similarity search.
    /// Empty for flat connectors (PDF, plain text) — edges inferred later.
    pub pre_defined_edges: Vec<PreDefinedEdge>,
}

/// A relationship between two chunks known before embedding.
/// Thread replies, function calls, calendar event attendees, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreDefinedEdge {
    /// source_uri of the target chunk.
    pub to_uri: String,
    /// Human-readable relationship label.
    pub label: String,
    /// Confidence that this relationship is real (0.0 - 1.0).
    pub relationship_probability: f64,
    /// Estimated token cost to traverse this edge.
    pub token_cost: i32,
}

impl NormalizedChunk {
    /// Convenience constructor for flat sources (no pre-defined edges).
    pub fn new(
        text: impl Into<String>,
        source_uri: impl Into<String>,
        domain: Domain,
        chunk_index: usize,
    ) -> Self {
        Self {
            text:               text.into(),
            metadata:           HashMap::new(),
            chunk_index,
            source_uri:         source_uri.into(),
            domain,
            pre_defined_edges:  vec![],
        }
    }
 
    /// Add a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
 
    /// Add a pre-defined edge to another chunk.
    pub fn with_edge(mut self, edge: PreDefinedEdge) -> Self {
        self.pre_defined_edges.push(edge);
        self
    }
 
    /// Returns true if this chunk has structured edges (skip similarity wiring).
    pub fn has_pre_defined_edges(&self) -> bool {
        !self.pre_defined_edges.is_empty()
    }
}

// ── FluvioConnector trait ─────────────────────────────────────────────────────
/// Every source connector implements this trait.
/// The connector's only job: given a source path/ID, produce normalized chunks.
/// The pipeline handles embedding, node creation, and edge wiring.
pub trait FluvioConnector: Send + Sync {
    /// Which domain this connector produces chunks for.
    fn domain(&self) -> Domain;
 
    /// Human-readable name for logging and CLI output.
    fn name(&self) -> &str;
 
    /// Extract normalized chunks from a source.
    /// `source` is interpreted by the connector:
    ///   PDF       → file path
    ///   Email     → "full" | "incremental" | a Gmail query string
    ///   Codebase  → directory path
    ///   Music     → file path or playlist URI
    fn extract(&self, source: &str) -> Result<Vec<NormalizedChunk>, ConnectorError>;
}

// ── Tests ─────────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::enums::Domain;
 
    #[test]
    fn test_normalized_chunk_new() {
        let chunk = NormalizedChunk::new(
            "Hello world",
            "gmail://message/abc123",
            Domain::Email,
            0,
        );
        assert_eq!(chunk.text, "Hello world");
        assert_eq!(chunk.source_uri, "gmail://message/abc123");
        assert_eq!(chunk.chunk_index, 0);
        assert!(chunk.metadata.is_empty());
        assert!(chunk.pre_defined_edges.is_empty());
        assert!(!chunk.has_pre_defined_edges());
    }
 
    #[test]
    fn test_normalized_chunk_with_metadata() {
        let chunk = NormalizedChunk::new("text", "uri", Domain::Email, 0)
            .with_metadata("sender", "alice@example.com")
            .with_metadata("subject", "Hello");
 
        assert_eq!(chunk.metadata.get("sender").unwrap(), "alice@example.com");
        assert_eq!(chunk.metadata.get("subject").unwrap(), "Hello");
    }
 
    #[test]
    fn test_normalized_chunk_with_edge() {
        let edge = PreDefinedEdge {
            to_uri:                   "gmail://message/reply123".to_string(),
            label:                    "reply_to".to_string(),
            relationship_probability: 1.0,
            token_cost:               1,
        };
 
        let chunk = NormalizedChunk::new("text", "uri", Domain::Email, 0)
            .with_edge(edge);
 
        assert!(chunk.has_pre_defined_edges());
        assert_eq!(chunk.pre_defined_edges.len(), 1);
        assert_eq!(chunk.pre_defined_edges[0].label, "reply_to");
    }
 
    #[test]
    fn test_pre_defined_edge_probability_range() {
        let edge = PreDefinedEdge {
            to_uri:                  "uri".to_string(),
            label:                   "test".to_string(),
            relationship_probability: 0.95,
            token_cost:              2,
        };
        assert!(edge.relationship_probability >= 0.0);
        assert!(edge.relationship_probability <= 1.0);
    }
 
    #[test]
    fn test_chunk_serialization() {
        let chunk = NormalizedChunk::new("test text", "pdf:///test.pdf#page=1", Domain::Pdf, 0)
            .with_metadata("page", "1");
 
        let json  = serde_json::to_string(&chunk).unwrap();
        let back: NormalizedChunk = serde_json::from_str(&json).unwrap();
 
        assert_eq!(back.text, "test text");
        assert_eq!(back.source_uri, "pdf:///test.pdf#page=1");
        assert_eq!(back.metadata.get("page").unwrap(), "1");
    }
}