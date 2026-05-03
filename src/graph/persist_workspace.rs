//! Load/save workspace snapshots as [`DomainGraph`] (FluvioGraph) JSON only.

use std::collections::HashSet;
use std::path::Path;

use crate::graph::enums::GraphError;
use crate::graph::fluvio_graph::FluvioGraph;
use crate::graph::structs::{DomainGraph, Node, NodeId};

/// Load a FluvioGraph snapshot from `path` into `graph` (same shape as [`DomainGraph::save`]).
pub fn load_workspace_graph(path: &str, graph: &mut DomainGraph) -> Result<(), GraphError> {
    graph.load(path)
}

/// Save a filtered subgraph (same JSON shape as a full [`DomainGraph`] snapshot).
pub fn save_domain_graph_filtered<P>(
    full: &DomainGraph,
    path: &str,
    pred: P,
) -> Result<(), GraphError>
where
    P: Fn(&Node) -> bool,
{
    let ids: HashSet<NodeId> = full.nodes.values().filter(|n| pred(n)).map(|n| n.id).collect();

    let mut sub = DomainGraph::new(full.id.clone(), full.domain.clone());
    if ids.is_empty() {
        return sub.save(path);
    }

    for id in &ids {
        if let Some(n) = full.nodes.get(id) {
            sub.insert_node(n.clone())?;
        }
    }

    for from_id in &ids {
        if let Some(edges) = full.adj.get(from_id) {
            for e in edges {
                if ids.contains(&e.to) {
                    sub.insert_edge(e.clone())?;
                }
            }
        }
    }

    sub.save(path)
}

/// Load if file exists; otherwise no-op. Returns whether a file was loaded.
pub fn load_workspace_graph_if_exists(path: &str, graph: &mut DomainGraph) -> Result<bool, GraphError> {
    if !Path::new(path).is_file() {
        return Ok(false);
    }
    load_workspace_graph(path, graph)?;
    Ok(true)
}
