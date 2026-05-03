
// Each domain graph is a separate fluvio_graph-* crate with its own schema, but they all implement a shared 
// Trait from fluvio-graph-core.
#![allow(dead_code)]

use std::collections::HashMap;
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};
use serde_json::to_string_pretty;

use crate::graph::enums::{Domain, GraphError, GraphQuery, GraphEvent, GraphResult, NodeKind};
use crate::graph::structs::{NodeId, EdgeId, GraphId, Node, Edge, DomainGraph};

pub trait FluvioGraph {
    fn graph_id(&self) -> &GraphId;
    fn domain(&self) -> &Domain;

    fn insert_node(&mut self, node: Node) -> Result<NodeId, GraphError>;
    fn insert_edge(&mut self, edge: Edge) -> Result<EdgeId, GraphError>;

    fn get_node(&self, node_id: &NodeId) -> Option<&Node>;
    fn get_edges_from(&self, node_id: &NodeId) -> &[Edge];

    fn delete_node(&mut self, id: NodeId) -> bool;
    fn update_node(&mut self, id: NodeId, node: Node) -> Result<(), GraphError>;


    fn query(&self, query: GraphQuery) -> GraphResult;
    fn subscribe(&self, event: GraphEvent) -> broadcast::Receiver<GraphEvent>;

    fn save(&self, path: &str) -> Result<(), GraphError>;
    fn load(&mut self, path: &str) -> Result<(), GraphError>;

    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
}

impl FluvioGraph for DomainGraph {

    fn graph_id(&self) -> &GraphId { &self.id }

    fn domain(&self) -> &Domain { &self.domain }

    fn insert_node(&mut self, node: Node) -> Result<NodeId, GraphError> {
        let id = node.id;
        self.nodes.insert(id, node);
        self.adj.entry(id).or_default();
        let _ = self.tx.send(GraphEvent::NodeInserted(id));
        Ok(id)
    }
 
    fn insert_edge(&mut self, edge: Edge) -> Result<EdgeId, GraphError> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(GraphError::NodeNotFound(edge.from));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(GraphError::NodeNotFound(edge.to));
        }
        let eid = edge.id;
        let from = edge.from;
        let vec = self.adj.entry(from).or_default();
        let idx = vec.len();
        vec.push(edge);
        self.edge_index.insert(eid, (from, idx));
        let _ = self.tx.send(GraphEvent::EdgeInserted(eid));
        Ok(eid)
    }

    fn get_node(&self, node_id: &NodeId) -> Option<&Node> { self.nodes.get(node_id) }

    fn get_edges_from(&self, node_id: &NodeId) -> &[Edge] {
        self.adj.get(node_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    fn delete_node(&mut self, id: NodeId) -> bool {
        if self.nodes.remove(&id).is_none() {
            return false;
        }

        self.adj.remove(&id);

        for edge in self.adj.values_mut() {
            edge.retain(|e| e.to != id);
        }
        let _ = self.tx.send(GraphEvent::NodeDeleted(id));
        true
    }

    fn update_node(&mut self, id: NodeId, node: Node) -> Result<(), GraphError> {
        let slot = self.nodes.get_mut(&id).ok_or(GraphError::NodeNotFound(id))?;
        *slot = node;
        let _ = self.tx.send(GraphEvent::NodeUpdated(id));
        Ok(())
    }

    fn query(&self, query: GraphQuery) -> GraphResult {
        match query {
            GraphQuery::Neighbors { root, depth } => {
                GraphResult::Nodes(self.neighbor_depth(root, depth))
            }

            GraphQuery::ShortestPath { from, to } => {
                GraphResult::Path(self.shorted_path(from, to))
            }

            GraphQuery::Filter(pred) => {
                let nodes = self.nodes
                      .values()
                      .filter(|n| Self::matches_predicate(n, &pred))
                      .cloned()
                      .collect();
                GraphResult::Nodes(nodes)
            }

            GraphQuery::Bfs { root } => {
                let ids = self.bfs_internal(root);
                let nodes = ids
                    .into_iter()
                    .filter_map(|id| self.nodes.get(&id).cloned())
                    .collect();
                GraphResult::Nodes(nodes)
            }

            GraphQuery::SimilarTo { embedding, top_k } => {
                GraphResult::Scored(self.similarity_search(&embedding, top_k))
            }

            GraphQuery::RefsForDomain(domain) => {
                let nodes = self
                    .nodes
                    .values()
                    .filter(|n| {
                        if let NodeKind::ExternalRef(r) = &n.kind {
                            &r.domain == &domain
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();
                GraphResult::Nodes(nodes)
            }
        }
    }

    fn subscribe(&self, _event: GraphEvent) -> broadcast::Receiver<GraphEvent> { self.tx.subscribe() }

    fn node_count(&self) -> usize { self.nodes.len() }
    fn edge_count(&self) -> usize { self.adj.values().map(|v| v.len()).sum() }

    fn save(&self, path: &str) -> Result<(), GraphError> {
        #[derive(Serialize)]
        struct Snapshot<'a> {
            id: &'a GraphId,
            domain: &'a Domain,
            nodes: &'a HashMap<NodeId, Node>,
            adj: &'a HashMap<NodeId, Vec<Edge>>,
        }

        let snap = Snapshot {
            id: &self.id,
            domain: &self.domain,
            nodes: &self.nodes,
            adj: &self.adj,
        };

        let json = to_string_pretty(&snap).map_err(|e| GraphError::SerializationError(e.to_string()))?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load(&mut self, path: &str) -> Result<(), GraphError> {
        #[derive(Deserialize)]
        struct Snapshot {
            nodes: HashMap<NodeId, Node>,
            adj: HashMap<NodeId, Vec<Edge>>,
        }
        let json = std::fs::read_to_string(path)?;
        let snap: Snapshot = serde_json::from_str(&json)
            .map_err(|e| GraphError::SerializationError(e.to_string()))?;
        self.nodes = snap.nodes;
        self.adj = snap.adj;
        // Rebuild edge_index from loaded adj.
        self.edge_index.clear();
        for (from, edges) in &self.adj {
            for (idx, edge) in edges.iter().enumerate() {
                self.edge_index.insert(edge.id, (*from, idx));
            }
        }
        Ok(())
    }
}

impl DomainGraph {
    /// Empty all nodes and edges (workspace reset).
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.adj.clear();
        self.edge_index.clear();
    }

    /// Deletes every node whose `metadata[key] == value`, and all incident edges. Returns how many nodes were removed.
    pub fn remove_nodes_by_metadata(&mut self, key: &str, value: &str) -> usize {
        let ids: Vec<NodeId> = self
            .nodes
            .values()
            .filter(|n| n.metadata.get(key).map(|v| v == value).unwrap_or(false))
            .map(|n| n.id)
            .collect();
        let count = ids.len();
        for id in ids {
            let _ = FluvioGraph::delete_node(self, id);
        }
        count
    }
}
