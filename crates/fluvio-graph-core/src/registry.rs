#![allow(dead_code)]
//-----------------------------------------------------
// Meta Graph Registry
//-----------------------------------------------------
// Owns all domain graph and the the meta graph.
// Agents interact with their domain graph through this registry.
// Cross-Domain queries go through the meta graph.
use std::collections::HashMap;
use crate::graph::FluvioGraph;
use fluvio_types::{GraphId, DomainGraph};
use fluvio_types::{NodeId, Node, ExternalRef, Edge, EdgeId};
use fluvio_types::{NodeKind, Domain, GraphError};

pub struct GraphRegistry {
    graphs: HashMap<GraphId, Box<dyn FluvioGraph + Send + Sync>>,
    meta: DomainGraph,
}

impl GraphRegistry {
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            meta: DomainGraph::new(GraphId::new("meta"), Domain::Custom("meta".into())),
        }
    }

    pub fn register<G: FluvioGraph + Send + Sync + 'static>(&mut self, graph: G) {
        self.graphs.insert(graph.graph_id().clone(), Box::new(graph));
    }

    pub fn get(&self, id: &GraphId) -> Option<&(dyn FluvioGraph + Send + Sync + '_)> {
        self.graphs.get(id).map(|g: &Box<dyn FluvioGraph + Send + Sync>| g.as_ref())
    }

    pub fn get_mut(&mut self, id: &GraphId) -> Option<&mut (dyn FluvioGraph + Send + Sync + '_)> {
        match self.graphs.get_mut(id) {
            Some(g) => Some(&mut **g),
            None => None,
        }
    }

    pub fn meta(&self) -> &DomainGraph { &self.meta }
    pub fn meta_mut(&mut self) -> &mut DomainGraph { &mut self.meta }

    /// Link the same real-world entity across two domain graphs via the meta-graph
    /// Creates ExternalRef nodes in the meta-graph and an edge between them.
    pub fn link_cross_domain(
        &mut self,
        graph_a: &GraphId,
        node_a: &NodeId,
        domain_a: &Domain,
        graph_b: &GraphId,
        node_b: &NodeId,
        domain_b: &Domain,
        relationship: &str,
    ) -> Result<(), GraphError> {

        let ref_a_id = NodeId::from_content("external_ref", &format!("{}::{}", graph_a.0, node_a.0));
        let ref_b_id = NodeId::from_content("external_ref", &format!("{}::{:?}", graph_b.0, node_b.0));

        // Insert externalRef nodes meta-graph if not already there.
        if self.meta.get_node(&ref_a_id).is_none() {
            self.meta.insert_node(Node {
                id: ref_a_id,
                domain: domain_a.clone(),
                source_uri: format!("{}::{:?}", graph_a.0, node_a.0),
                source_text: String::new(),
                embeddings: vec![],
                metadata: HashMap::new(),
                kind: NodeKind::ExternalRef(ExternalRef {
                    graph_id: graph_a.clone(),
                    node_id: node_a.clone(),
                    domain: domain_a.clone(),
                }),
                zone: 0,
            });
        }

        if self.meta.get_node(&ref_b_id).is_none() {
            self.meta.insert_node(Node {
                id: ref_b_id,
                domain: domain_b.clone(),
                source_uri: format!("{}::{:?}", graph_b.0, node_b.0),
                source_text: String::new(),
                embeddings: vec![],
                metadata: HashMap::new(),
                kind: NodeKind::ExternalRef(ExternalRef {
                    graph_id: graph_b.clone(),
                    node_id: node_b.clone(),
                    domain: domain_b.clone(),
                }),
                zone: 0,
            });
        }

        // Edge between them in the meta-graph.
        self.meta.insert_edge(Edge {
            id: EdgeId::new(),
            from: ref_a_id.clone(),
            to: ref_b_id,
            token: 1,
            relationship_probability: 0.95,
            label: relationship.to_string(),
            metadata: HashMap::new(),
        });
     
        Ok(())
    }

    pub fn save_all(&self, dir: &str) -> Result<(), GraphError> {
        std::fs::create_dir_all(dir)?;
        for (id, graph) in &self.graphs {
            graph.save(&format!("{}/{}.json", dir, id.0))?;
        }
        self.meta.save(&format!("{}/meta.json", dir))?;
        Ok(())
    }

}