#![allow(dead_code)]
use tokio::sync::broadcast;
use std::cmp::Reverse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use crate::graph::enums::{Domain, NodeKind, GraphEvent, NodePredicate, NodeKindFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Content-addressed: deterministic from source type + canonical text.
    /// Two sources extracting "Peter Steinberger" as a Person → identical NodeId.
    pub fn from_content(source_type: &str, canonical_text: &str) -> Self {

        let input = format!("{}::{}", source_type, canonical_text.trim().to_lowercase());

        let hash = blake3::hash(input.as_bytes());

        // Take first 16 bytes of the blake3 hash and interpret as a UUID v4-shaped value.
        let bytes: [u8; 16] = hash.as_bytes()[..16].try_into().unwrap();

        Self(Uuid::from_bytes(bytes))
    }

    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub Uuid);


impl EdgeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphId(pub String);

impl GraphId {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

/// Cross-domain pointer — lives in the MetaGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRef {
    pub graph_id: GraphId,
    pub node_id: NodeId,
    pub domain: Domain,
}

// -------------------------------------------------------------
// Node and Edge structs
// -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub domain: Domain,
    pub source_uri: String, // file path, message id, spotify uri, etc.
    pub source_text: String, // canonical extractable text
    pub embeddings: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,

    // Token cost of traversing this edge.
    pub token: i32,
    //Confidence this relationship is real. Used as heuristic cost += (1 - probability) * 1000.
    pub relationship_probability: f64,
    pub label: String, // "author_by", "reference", "co-occurs_with, etc."
    pub metadata: HashMap<String, String>,
}

// -------------------------------------------------------------
// Domain Graph - Concret Implementation of FluvioGraph Trait
// -------------------------------------------------------------
pub struct DomainGraph {
    pub id: GraphId,
    pub domain: Domain,
    pub nodes: HashMap<NodeId, Node>,
    pub adj: HashMap<NodeId, Vec<Edge>>, // outgoing edges per node.
    pub edge_index: HashMap<EdgeId, (NodeId, usize)>, // edge id -> (from_node, index in adj vec)
    pub tx: broadcast::Sender<GraphEvent>,
}

impl DomainGraph {
    pub fn new(id: GraphId, domain: Domain) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            id,
            domain,
            nodes: HashMap::new(),
            adj: HashMap::new(),
            edge_index: HashMap::new(),
            tx,
        }
    }

    // --- Internal traversal helpers --- 
    pub fn bfs_internal(&self, start: NodeId) -> Vec<NodeId> {
        if !self.nodes.contains_key(&start) {
            return vec![];
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut order = Vec::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            order.push(current);
            for edge in self.adj.get(&current).map(|v| v.as_slice()).unwrap_or(&[]) {
                if visited.insert(edge.to) {
                    queue.push_back(edge.to);
                }
            }
        }
        order
    }

    pub fn neighbor_depth(&self, root: NodeId, depth: usize) -> Vec<Node> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        visited.insert(root);
        queue.push_back((root, 0usize));

        while let Some((current, d)) = queue.pop_front() {
            if d >= depth {
                continue;
            }

            for edge in self.adj.get(&current).map(|v| v.as_slice()).unwrap_or(&[]) {
                if visited.insert(edge.to) {
                    if let Some(n) = self.nodes.get(&edge.to) {
                        result.push(n.clone());
                    }
                    queue.push_back((edge.to, d + 1));
                }
            }
        }
        result
    }


    /// Dual-weight Dijkstra: cost = token + ( 1- prob) * 1000
    pub fn shorted_path(&self, start: NodeId, goal: NodeId) -> Option<Vec<NodeId>> {
        #[derive(PartialEq, Eq)]
        struct State {
            cost: i64, 
            node: NodeId,
            path: Vec<NodeId>,
        }

        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for State {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.cost.cmp(&other.cost)
            }
        }

        let mut visited = HashSet::new();
        let mut heap = BinaryHeap::new();
        heap.push(Reverse(State {
            cost: 0,
            node: start,
            path: vec![start],
        }));

        while let Some(Reverse(State { cost, node, path })) = heap.pop() {
            if node == goal {
                return Some(path);
            }
            if !visited.insert(node) {
                continue;
            }
            
            for edge in self.adj.get(&node).map(|v| v.as_slice()).unwrap_or(&[]) {
                if visited.contains(&edge.to) {
                    continue;
                }

                let uncertainty = ((1.0 - edge.relationship_probability) * 1000.0) as i64;
                let new_cost = cost + edge.token as i64 + uncertainty;
                let mut new_path = path.clone();

                new_path.push(edge.to);
                heap.push(Reverse(State {
                    cost: new_cost,
                    node: edge.to,
                    path: new_path,
                }));
            }
        }

        None
    }

    pub fn similarity_search(&self, query: &[f32], top_k: usize) -> Vec<(NodeId, f32)> {
        let query_norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();

        if query_norm == 0.0 {
            return vec![];
        }

        let mut scores: Vec<(NodeId, f32)> = self.nodes.values()
            .map(|node| {
                let dot: f32 = query.iter().zip(&node.embeddings).map(|(a, b)| a * b).sum();
                let node_norm: f32 = node.embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
                let sim = if node_norm == 0.0 { 0.0 } else { dot / (query_norm * node_norm) };
                (node.id, sim)
            }).collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    pub fn matches_predicate(node: &Node, pred: &NodePredicate) -> bool {
        match pred {
            NodePredicate::ByDomain(d) => &node.domain == d,
            NodePredicate::ByKind(k) => match (k, &node.kind) {
                (NodeKindFilter::Entity, NodeKind::Entity) => true,
                (NodeKindFilter::Topic, NodeKind::Topic) => true,
                (NodeKindFilter::Artifcat, NodeKind::Artifcat) => true,
                (NodeKindFilter::Event, NodeKind::Event) => true,
                (NodeKindFilter::Conversation, NodeKind::Conversation) => true,
                (NodeKindFilter::ExternalRef, NodeKind::ExternalRef(_)) => true,
                _ => false,
            },
            NodePredicate::ByMetadata { key, value } => {
                node.metadata.get(key).map(|v| v == value).unwrap_or(false)
            },
            NodePredicate::And(left, right) => {
                Self::matches_predicate(node, left) && Self::matches_predicate(node, right)
            },
            NodePredicate::Or(left, right) => {
                Self::matches_predicate(node, left) || Self::matches_predicate(node, right)
            },
        }
    }

}

