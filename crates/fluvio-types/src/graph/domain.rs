//! `DomainGraph` — the in-memory graph engine.
//!
//! This is the concrete implementation of the Fluvio graph.
//! It is the direct heir of `DomainGraph` from the monolith `src/graph/structs.rs`.
//!
//! ## Design notes
//! - Adjacency list representation: `adj: HashMap<NodeId, Vec<Edge>>`
//! - Edge index for O(1) lookup by EdgeId: `edge_index: HashMap<EdgeId, (NodeId, usize)>`
//! - Broadcast channel for in-memory event subscribers (live sync, cache invalidation)
//! - `Clone` is manually implemented to create a fresh channel rather than sharing the sender
//!
//! ## In-memory vs persistent
//! `DomainGraph` lives in RAM during a session.
//! Persistence is handled by the storage layer in `services/fluvio-graph/src/storage/surreal.rs`.
//! On session open the graph is warm-loaded from SurrealDB via `SurrealStorage::get_user_nodes`.

#![allow(dead_code)]

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use tokio::sync::broadcast;

use crate::graph::edge::Edge;
use crate::graph::enums::{Domain, GraphEvent, NodeKind, NodeKindFilter, NodePredicate};
use crate::graph::ids::{EdgeId, GraphId, NodeId};
use crate::graph::node::Node;

// ── DomainGraph ───────────────────────────────────────────────────────────────

pub struct DomainGraph {
    pub id:         GraphId,
    pub domain:     Domain,
    pub nodes:      HashMap<NodeId, Node>,
    /// Outgoing edges per source node.
    pub adj:        HashMap<NodeId, Vec<Edge>>,
    /// O(1) edge lookup: EdgeId → (from NodeId, index in adj vec).
    pub edge_index: HashMap<EdgeId, (NodeId, usize)>,
    /// Broadcast channel for `GraphEvent` subscribers.
    pub tx:         broadcast::Sender<GraphEvent>,
}

// Manual Clone: create a fresh channel instead of sharing the sender.
impl Clone for DomainGraph {
    fn clone(&self) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            id:         self.id.clone(),
            domain:     self.domain.clone(),
            nodes:      self.nodes.clone(),
            adj:        self.adj.clone(),
            edge_index: self.edge_index.clone(),
            tx,
        }
    }
}

impl DomainGraph {
    // ── Construction ──────────────────────────────────────────────────────────

    pub fn new(id: GraphId, domain: Domain) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            id,
            domain,
            nodes:      HashMap::new(),
            adj:        HashMap::new(),
            edge_index: HashMap::new(),
            tx,
        }
    }

    /// Subscribe to graph mutation events.
    pub fn subscribe(&self) -> broadcast::Receiver<GraphEvent> {
        self.tx.subscribe()
    }

    // ── Mutation ──────────────────────────────────────────────────────────────

    pub fn insert_node(&mut self, node: Node) {
        let id = node.id;
        self.nodes.insert(id, node);
        self.adj.entry(id).or_default();
        let _ = self.tx.send(GraphEvent::NodeInserted(id));
    }

    pub fn insert_edge(&mut self, edge: Edge) {
        let from = edge.from;
        let eid  = edge.id;
        let vec  = self.adj.entry(from).or_default();
        let idx  = vec.len();
        vec.push(edge);
        self.edge_index.insert(eid, (from, idx));
        let _ = self.tx.send(GraphEvent::EdgeInserted(eid));
    }

    pub fn remove_node(&mut self, id: NodeId) -> Option<Node> {
        let node = self.nodes.remove(&id);
        if node.is_some() {
            self.adj.remove(&id);
            let _ = self.tx.send(GraphEvent::NodeDeleted(id));
        }
        node
    }

    // ── Subgraph extraction ───────────────────────────────────────────────────

    /// Return a new graph containing only `subset` nodes and edges whose
    /// both endpoints lie in `subset`. No graph events are fired.
    pub fn subgraph_closed<I: IntoIterator<Item = NodeId>>(&self, subset: I) -> Self {
        let ids: HashSet<NodeId> = subset.into_iter().collect();
        let mut sub = DomainGraph::new(self.id.clone(), self.domain.clone());

        for id in &ids {
            if let Some(n) = self.nodes.get(id) {
                sub.nodes.insert(*id, n.clone());
                sub.adj.entry(*id).or_default();
            }
        }
        for id in &ids {
            let Some(edges) = self.adj.get(id) else { continue };
            for e in edges {
                if !ids.contains(&e.to) { continue; }
                let vec = sub.adj.entry(e.from).or_default();
                let idx = vec.len();
                vec.push(e.clone());
                sub.edge_index.insert(e.id, (e.from, idx));
            }
        }
        sub
    }

    // ── Traversal ─────────────────────────────────────────────────────────────

    /// BFS from `start` — returns node IDs in visit order.
    pub fn bfs_internal(&self, start: NodeId) -> Vec<NodeId> {
        if !self.nodes.contains_key(&start) { return vec![]; }

        let mut visited = HashSet::new();
        let mut queue   = VecDeque::new();
        let mut order   = Vec::new();

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

    /// All nodes within `depth` hops of `root`, exclusive of root itself.
    pub fn neighbor_depth(&self, root: NodeId, depth: usize) -> Vec<Node> {
        let mut visited = HashSet::new();
        let mut queue   = VecDeque::new();
        let mut result  = Vec::new();

        visited.insert(root);
        queue.push_back((root, 0usize));

        while let Some((current, d)) = queue.pop_front() {
            if d >= depth { continue; }
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

    // ── Shortest path (dual-weight Dijkstra) ──────────────────────────────────

    /// Dual-weight Dijkstra:
    /// `cost = token + (1.0 - relationship_probability) * 1000`
    ///
    /// Returns the lowest-cost path from `start` to `goal`, or `None` if
    /// no path exists.
    pub fn shortest_path(&self, start: NodeId, goal: NodeId) -> Option<Vec<NodeId>> {
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
        let mut heap    = BinaryHeap::new();
        heap.push(Reverse(State { cost: 0, node: start, path: vec![start] }));

        while let Some(Reverse(State { cost, node, path })) = heap.pop() {
            if node == goal { return Some(path); }
            if !visited.insert(node) { continue; }

            for edge in self.adj.get(&node).map(|v| v.as_slice()).unwrap_or(&[]) {
                if visited.contains(&edge.to) { continue; }

                let uncertainty = ((1.0 - edge.relationship_probability) * 1000.0) as i64;
                let new_cost    = cost + edge.token as i64 + uncertainty;
                let mut new_path = path.clone();
                new_path.push(edge.to);

                heap.push(Reverse(State { cost: new_cost, node: edge.to, path: new_path }));
            }
        }
        None
    }

    // ── Similarity search (in-memory) ─────────────────────────────────────────

    /// Cosine similarity over all nodes in memory.
    /// For large graphs prefer SurrealDB's `vector::similarity::cosine` index.
    pub fn similarity_search(&self, query: &[f32], top_k: usize) -> Vec<(NodeId, f32)> {
        let query_norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
        if query_norm == 0.0 { return vec![]; }

        let mut scores: Vec<(NodeId, f32)> = self.nodes.values()
            .map(|node| {
                let dot: f32 = query.iter().zip(&node.embeddings).map(|(a, b)| a * b).sum();
                let node_norm: f32 = node.embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
                let sim = if node_norm == 0.0 { 0.0 } else { dot / (query_norm * node_norm) };
                (node.id, sim)
            })
            .collect();

        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    // ── Predicate matching ────────────────────────────────────────────────────

    /// Evaluate a `NodePredicate` tree against a single node.
    pub fn matches_predicate(node: &Node, pred: &NodePredicate) -> bool {
        match pred {
            NodePredicate::ByDomain(d) => &node.domain == d,
            NodePredicate::ByKind(k)   => match (k, &node.kind) {
                (NodeKindFilter::Entity,       NodeKind::Entity)       => true,
                (NodeKindFilter::Topic,        NodeKind::Topic)        => true,
                (NodeKindFilter::Artifcat,     NodeKind::Artifcat)     => true,
                (NodeKindFilter::Event,        NodeKind::Event)        => true,
                (NodeKindFilter::Conversation, NodeKind::Conversation) => true,
                (NodeKindFilter::ExternalRef,  NodeKind::ExternalRef(_)) => true,
                _ => false,
            },
            NodePredicate::ByMetadata { key, value } => {
                node.metadata.get(key).map(|v| v == value).unwrap_or(false)
            },
            NodePredicate::And(l, r) => {
                Self::matches_predicate(node, l) && Self::matches_predicate(node, r)
            },
            NodePredicate::Or(l, r) => {
                Self::matches_predicate(node, l) || Self::matches_predicate(node, r)
            },
        }
    }

    /// Filter all nodes by predicate and return matching nodes.
    pub fn filter(&self, pred: &NodePredicate) -> Vec<&Node> {
        self.nodes.values()
            .filter(|n| Self::matches_predicate(n, pred))
            .collect()
    }

    // ── Stats ─────────────────────────────────────────────────────────────────

    pub fn node_count(&self) -> usize { self.nodes.len() }

    pub fn edge_count(&self) -> usize {
        self.adj.values().map(|v| v.len()).sum()
    }
}