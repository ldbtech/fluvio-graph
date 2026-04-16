//! Node/edge schema, persistence, and graph operations.

mod document;
mod embeddings;
mod pdf;
pub mod fluvio_graph;
pub mod enums;
pub mod structs;
pub mod graph_registry;

pub use document::{Document, TextChunk};
pub use embeddings::EmbeddingContext;
pub use pdf::Pdf;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Node {0} not found")]
    NodeNotFound(NodeId),
    #[error("embedding failed: {0}")]
    EmbeddingFailed(String),
}

pub type NodeId = Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub embeddings: Vec<f32>,
    pub source_text: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub token: i32,
    pub relationship_probability: f64,
}

pub struct Graph {
    pub nodes: HashMap<NodeId, Node>,
    pub adj_list: HashMap<NodeId, Vec<Edge>>,
    pub embed_ctx: Arc<std::sync::Mutex<EmbeddingContext>>,
}

impl Graph {
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        #[derive(Serialize)]
        struct Snapshot<'a> {
            nodes: &'a HashMap<NodeId, Node>,
            adj_list: &'a HashMap<NodeId, Vec<Edge>>,
        }

        let snap = Snapshot {
            nodes: &self.nodes,
            adj_list: &self.adj_list,
        };
        let json = to_string_pretty(&snap)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(&mut self, path: &str) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct Snapshot {
            nodes: HashMap<NodeId, Node>,
            adj_list: HashMap<NodeId, Vec<Edge>>,
        }

        let json = std::fs::read_to_string(path)?;
        let snap: Snapshot = serde_json::from_str(&json)?;
        self.nodes = snap.nodes;
        self.adj_list = snap.adj_list;
        Ok(())
    }

    pub fn similarity_search(&self, query: &[f32], top_k: usize) -> Vec<(NodeId, f32)> {
        let query_norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();

        if query_norm == 0.0 {
            return vec![];
        }

        let mut scores: Vec<(NodeId, f32)> = self
            .nodes
            .values()
            .map(|node| {
                let dot: f32 = query.iter().zip(&node.embeddings).map(|(a, b)| a * b).sum();
                let node_norm: f32 = node.embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
                let sim = if node_norm == 0.0 {
                    0.0
                } else {
                    dot / (query_norm * node_norm)
                };
                (node.id, sim)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    pub fn new(embed_ctx: Arc<std::sync::Mutex<EmbeddingContext>>) -> Self {
        Self {
            nodes: HashMap::new(),
            adj_list: HashMap::new(),
            embed_ctx,
        }
    }

    pub fn add_node(&mut self, data: Box<dyn Document>) -> Result<NodeId, GraphError> {
        let text: String = data.extracted_text().to_string();

        let embeddings = {
            let mut ctx = self.embed_ctx.lock().unwrap();
            ctx.embed(&text)?
        };

        let id = Uuid::new_v4();

        self.nodes.insert(
            id,
            Node {
                id,
                embeddings,
                source_text: text,
                metadata: data.metadata(),
            },
        );

        self.adj_list.entry(id).or_default();
        Ok(id)
    }

    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        token: i32,
        relationship_probability: f64,
    ) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&from) {
            return Err(GraphError::NodeNotFound(from));
        }

        if !self.nodes.contains_key(&to) {
            return Err(GraphError::NodeNotFound(to));
        }

        self.adj_list.entry(from).or_default().push(Edge {
            from,
            to,
            token,
            relationship_probability,
        });

        Ok(())
    }

    pub fn delete_node(&mut self, id: NodeId) -> bool {
        if self.nodes.remove(&id).is_none() {
            return false;
        }

        self.adj_list.remove(&id);

        for edges in self.adj_list.values_mut() {
            edges.retain(|e| e.to != id);
        }

        true
    }

    pub fn update_node(&mut self, id: NodeId, data: Box<dyn Document>) -> Result<(), GraphError> {
        let text = data.extracted_text().to_string();

        let embeddings = self.embed_ctx.lock().unwrap().embed(&text)?;

        let node = self.nodes.get_mut(&id).ok_or(GraphError::NodeNotFound(id))?;

        node.embeddings = embeddings;
        node.source_text = text;

        Ok(())
    }

    /// BFS — returns node IDs in visit order from `start`.
    pub fn bfs(&self, start: NodeId) -> Vec<NodeId> {
        if !self.nodes.contains_key(&start) {
            return vec![];
        }
        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        let mut order = Vec::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            order.push(current);
            if let Some(edges) = self.adj_list.get(&current) {
                for edge in edges {
                    if visited.insert(edge.to) {
                        queue.push_back(edge.to);
                    }
                }
            }
        }
        order
    }

    /// Weighted traversal using the dual-weight edge: cost = token, heuristic = 1 - prob.
    /// Returns the lowest-combined-weight path from `start` to `goal`.
    pub fn weighted_traverse(&self, start: NodeId, goal: NodeId) -> Option<Vec<NodeId>> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        #[derive(PartialEq)]
        struct State {
            cost: i64,
            node: NodeId,
            path: Vec<NodeId>,
        }
        impl Eq for State {}
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

        let mut visited = std::collections::HashSet::new();
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
            if let Some(edges) = self.adj_list.get(&node) {
                for edge in edges {
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
        }
        None
    }
}
