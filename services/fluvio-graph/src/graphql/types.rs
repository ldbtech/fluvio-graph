//! GraphQL types for fluvio-graph subgraph.
//!
//! Maps internal Rust types → async-graphql object types.
//! These are what Apollo Router and clients see — never raw fluvio-types structs.

use async_graphql::*;
use std::collections::HashMap;

// ── GqlDomain ─────────────────────────────────────────────────────────────────

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GqlDomain {
    Pdf,
    Email,
    Whatsapp,
    Calendar,
    Codebase,
    Web,
    Custom,
}

impl From<&fluvio_types::Domain> for GqlDomain {
    fn from(d: &fluvio_types::Domain) -> Self {
        match d {
            fluvio_types::Domain::Pdf          => Self::Pdf,
            fluvio_types::Domain::Email        => Self::Email,
            fluvio_types::Domain::Whatsapp     => Self::Whatsapp,
            fluvio_types::Domain::Calendar     => Self::Calendar,
            fluvio_types::Domain::Codebase     => Self::Codebase,
            fluvio_types::Domain::Web          => Self::Web,
            fluvio_types::Domain::Custom(_)    => Self::Custom,
        }
    }
}

// ── GqlNodeKind ───────────────────────────────────────────────────────────────

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GqlNodeKind {
    Entity,
    Topic,
    /// Preserved typo from SurrealDB records — do not rename
    Artifcat,
    Event,
    Conversation,
    ExternalRef,
}

impl From<&fluvio_types::NodeKind> for GqlNodeKind {
    fn from(k: &fluvio_types::NodeKind) -> Self {
        match k {
            fluvio_types::NodeKind::Entity        => Self::Entity,
            fluvio_types::NodeKind::Topic         => Self::Topic,
            fluvio_types::NodeKind::Artifcat      => Self::Artifcat,
            fluvio_types::NodeKind::Event         => Self::Event,
            fluvio_types::NodeKind::Conversation  => Self::Conversation,
            fluvio_types::NodeKind::ExternalRef(_)=> Self::ExternalRef,
        }
    }
}

// ── GqlNode ───────────────────────────────────────────────────────────────────

/// A node in the Fluvio knowledge graph.
/// This is the primary entity — everything else references it.
#[derive(SimpleObject, Clone, Debug)]
#[graphql(complex)]
pub struct GqlNode {
    /// Stable UUID — content-addressed or random.
    pub id:          String,
    pub domain:      GqlDomain,
    /// Opaque locator: file path, message-id, GitHub URL, etc.
    pub source_uri:  String,
    /// Canonical extractable text used for embedding + LLM context.
    pub source_text: String,
    pub kind:        GqlNodeKind,
    /// Arbitrary key-value metadata.
    pub metadata:    Vec<GqlMetadataEntry>,
    /// Embedding vector is omitted from default responses — too large.
    /// Use embeddingDimensions to check if embedded.
    #[graphql(skip)]
    pub embeddings:  Vec<f32>,
}

#[ComplexObject]
impl GqlNode {
    /// Number of embedding dimensions. 0 means not yet embedded.
    async fn embedding_dimensions(&self) -> i32 {
        self.embeddings.len() as i32
    }

    /// Whether this node has been embedded.
    async fn is_embedded(&self) -> bool {
        !self.embeddings.is_empty()
    }
}

impl From<fluvio_types::Node> for GqlNode {
    fn from(n: fluvio_types::Node) -> Self {
        Self {
            id:          n.id.to_string(),
            domain:      GqlDomain::from(&n.domain),
            source_uri:  n.source_uri,
            source_text: n.source_text,
            kind:        GqlNodeKind::from(&n.kind),
            metadata:    n.metadata.into_iter()
                .map(|(k, v)| GqlMetadataEntry { key: k, value: v })
                .collect(),
            embeddings:  n.embeddings,
        }
    }
}

// ── GqlEdge ───────────────────────────────────────────────────────────────────

/// A directed, weighted relationship between two nodes.
#[derive(SimpleObject, Clone, Debug)]
pub struct GqlEdge {
    pub id:                       String,
    pub from:                     String,
    pub to:                       String,
    pub label:                    String,
    /// Approximate LLM token cost of this edge.
    pub token:                    i32,
    /// Confidence [0.0, 1.0] that the relationship is real.
    pub relationship_probability: f64,
    pub metadata:                 Vec<GqlMetadataEntry>,
}

impl From<fluvio_types::Edge> for GqlEdge {
    fn from(e: fluvio_types::Edge) -> Self {
        Self {
            id:                       e.id.to_string(),
            from:                     e.from.to_string(),
            to:                       e.to.to_string(),
            label:                    e.label,
            token:                    e.token,
            relationship_probability: e.relationship_probability,
            metadata:                 e.metadata.into_iter()
                .map(|(k, v)| GqlMetadataEntry { key: k, value: v })
                .collect(),
        }
    }
}

// ── GqlMetadataEntry ──────────────────────────────────────────────────────────

/// Key-value metadata pair (GraphQL can't return HashMap directly).
#[derive(SimpleObject, Clone, Debug)]
pub struct GqlMetadataEntry {
    pub key:   String,
    pub value: String,
}

// ── GqlScoredNode ─────────────────────────────────────────────────────────────

/// A node with a similarity score — returned by search queries.
#[derive(SimpleObject, Clone, Debug)]
pub struct GqlScoredNode {
    pub node:  GqlNode,
    pub score: f64,
}

// ── GqlPath ───────────────────────────────────────────────────────────────────

/// A sequence of nodes representing a path through the graph.
#[derive(SimpleObject, Clone, Debug)]
pub struct GqlPath {
    pub nodes: Vec<GqlNode>,
    pub found: bool,
}

// ── Input types ───────────────────────────────────────────────────────────────

/// Input for creating or updating a node.
#[derive(InputObject, Clone, Debug)]
pub struct GqlNodeInput {
    /// Optional — if omitted a random UUID is generated.
    pub id:          Option<String>,
    pub domain:      GqlDomain,
    pub source_uri:  String,
    pub source_text: String,
    pub kind:        GqlNodeKind,
    pub metadata:    Option<Vec<GqlMetadataInput>>,
    pub zone:        Option<i32>,
}

/// Input for creating or updating an edge.
#[derive(InputObject, Clone, Debug)]
pub struct GqlEdgeInput {
    pub from:                     String,
    pub to:                       String,
    pub label:                    String,
    pub token:                    Option<i32>,
    pub relationship_probability: Option<f64>,
}

/// Metadata key-value input.
#[derive(InputObject, Clone, Debug)]
pub struct GqlMetadataInput {
    pub key:   String,
    pub value: String,
}

/// Query tuning config — mirrors QueryConfig.
#[derive(InputObject, Clone, Debug)]
pub struct GqlQueryConfig {
    pub similarity_top_k:   Option<i32>,
    pub expansion_depth:    Option<i32>,
    pub max_subgraph_nodes: Option<i32>,
    /// 0 = own data, 1 = network
    pub max_zone:           Option<i32>,
}

impl From<GqlQueryConfig> for crate::query_context::QueryConfig {
    fn from(g: GqlQueryConfig) -> Self {
        let def = crate::query_context::QueryConfig::default();
        Self {
            similarity_top_k:   g.similarity_top_k.map(|v| v as usize).unwrap_or(def.similarity_top_k),
            expansion_depth:    g.expansion_depth.map(|v| v as usize).unwrap_or(def.expansion_depth),
            max_subgraph_nodes: g.max_subgraph_nodes.map(|v| v as usize).unwrap_or(def.max_subgraph_nodes),
            max_zone:           g.max_zone.map(|v| v as i16).unwrap_or(def.max_zone),
        }
    }
}

// ── Domain conversion helpers ─────────────────────────────────────────────────

impl From<GqlDomain> for fluvio_types::Domain {
    fn from(d: GqlDomain) -> Self {
        match d {
            GqlDomain::Pdf          => fluvio_types::Domain::Pdf,
            GqlDomain::Email        => fluvio_types::Domain::Email,
            GqlDomain::Whatsapp     => fluvio_types::Domain::Whatsapp,
            GqlDomain::Calendar     => fluvio_types::Domain::Calendar,
            GqlDomain::Codebase     => fluvio_types::Domain::Codebase,
            GqlDomain::Web          => fluvio_types::Domain::Web,
            GqlDomain::Custom       => fluvio_types::Domain::Custom("custom".into()),
        }
    }
}

impl From<GqlNodeKind> for fluvio_types::NodeKind {
    fn from(k: GqlNodeKind) -> Self {
        match k {
            GqlNodeKind::Entity       => fluvio_types::NodeKind::Entity,
            GqlNodeKind::Topic        => fluvio_types::NodeKind::Topic,
            GqlNodeKind::Artifcat     => fluvio_types::NodeKind::Artifcat,
            GqlNodeKind::Event        => fluvio_types::NodeKind::Event,
            GqlNodeKind::Conversation => fluvio_types::NodeKind::Conversation,
            GqlNodeKind::ExternalRef  => fluvio_types::NodeKind::Artifcat, // fallback
        }
    }
}