#![allow(dead_code)]
use crate::graph::{Graph, NodeId};
use uuid::Uuid;

pub struct KnowledgeGraphQuery<'a> {
    graph: &'a Graph,
}

impl<'a> KnowledgeGraphQuery<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    /// Find top-k semantically similar nodes to a query embedding
    pub fn search(&self, query_vec: &[f32], top_k: usize) -> Vec<QueryResult> {
        self.graph
            .similarity_search(query_vec, top_k)
            .into_iter()
            .map(|(id, score)| {
                let node = &self.graph.nodes[&id];
                QueryResult {
                    id,
                    score,
                    source_text: node.source_text.clone(),
                    metadata: node.metadata.clone(),
                }
            })
            .collect()
    }

    /// Traverse from a node, return ordered context window for LLM
    pub fn traverse_context(&self, start: Uuid, goal: Uuid) -> Option<Vec<QueryResult>> {
        self.graph
            .weighted_traverse(start, goal)
            .map(|path| {
                path.into_iter()
                    .map(|id| {
                        let node = &self.graph.nodes[&id];
                        QueryResult {
                            id,
                            score: 1.0,
                            source_text: node.source_text.clone(),
                            metadata: node.metadata.clone(),
                        }
                    })
                    .collect()
            })
    }

    /// BFS neighborhood — useful for "what's around this entity"
    pub fn neighborhood(&self, start: Uuid, depth: usize) -> Vec<QueryResult> {
        let all = self.graph.bfs(start);
        all.into_iter()
            .take(depth * 4) // rough depth approximation
            .map(|id| {
                let node = &self.graph.nodes[&id];
                QueryResult {
                    id,
                    score: 1.0,
                    source_text: node.source_text.clone(),
                    metadata: node.metadata.clone(),
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct QueryResult {
    pub id: NodeId,
    pub score: f32,
    pub source_text: String,
    pub metadata: std::collections::HashMap<String, String>,
}