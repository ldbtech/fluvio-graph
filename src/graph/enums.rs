use serde::{Deserialize, Serialize};
use crate::graph::structs::{
    Node, 
    NodeId, 
    EdgeId, 
    ExternalRef
};

/// Graph Results
#[derive(Debug, Clone)]
pub enum GraphResult {
    Nodes(Vec<Node>),
    Path(Option<Vec<NodeId>>),
    Scored(Vec<(NodeId, f32)>),
    Empty,
}

// Graph properties and operations
#[derive(Debug, thiserror::Error)]
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
}

/// This is for subscribing to graph events. (subscribe())
#[derive(Debug, Clone)]
pub enum GraphEvent {
    NodeInserted(NodeId),
    NodeUpdated(NodeId),
    NodeDeleted(NodeId),
    EdgeInserted(EdgeId),
}

// Graph Query: 
#[derive(Debug, Clone)]
pub enum GraphQuery {
    /// All neighbors of a node up to Depth N.
    Neighbors { root: NodeId, depth: usize },

    /// Weighted shortest path between two nodes (dual weight A*). 
    ShortestPath { from: NodeId, to: NodeId },

    /// All nodes mathching a predicate.
    Filter(NodePredicate),

    /// BFS to visit order from root.
    Bfs { root: NodeId},

    /// Semantic nearest neghbors by cosine similarity.
    SimilarTo { embedding: Vec<f32>, top_k: usize },

    /// All externalRef nodes pointing to a given domain 
    RefsForDomain(Domain),

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Entity,                     // person, organization, location, product, etc.
    Topic,                      // concept, idea, theme, category, etc.
    Artifcat,                   // document, file, image, video, etc.
    Event,                      // meeting, appointment, deadline, etc.
    Conversation,                // chat, conversation, discussion, etc.
    ExternalRef(ExternalRef),   // pointer to another graph (meta-graph only)
}

//// Node Predicate:
#[derive(Debug, Clone)]
pub enum NodePredicate {
    ByDomain(Domain),
    ByKind(NodeKindFilter),
    ByMetadata { key: String, value: String },
    And(Box<NodePredicate>, Box<NodePredicate>),
    Or(Box<NodePredicate>, Box<NodePredicate>),
}

#[derive(Debug, Clone)]
pub enum NodeKindFilter {
    Entity,
    Topic,
    Artifcat,
    Event,
    Conversation,
    ExternalRef,
}


// Domain
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    Pdf,
    Email,
    Whatsapp,
    Music,
    Calendar,
    Codebase,
    Web,
    Custom(String),
}