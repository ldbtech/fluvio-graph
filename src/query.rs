#![allow(dead_code)]
use crate::graph::{Graph, NodeId};
use uuid::Uuid;

fn truncate_for_prompt(s: &str, max_chars: usize) -> String {
    let n = s.chars().count();
    if n <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect::<String>() + "…"
}

pub struct KnowledgeGraphQuery<'a> {
    graph: &'a Graph,
}

impl<'a> KnowledgeGraphQuery<'a> {
    pub fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    /// Semantic seeds plus outgoing edges (probability, token cost, neighbor text) for LLM context.
    pub fn search_with_relational_context(
        &self,
        query_vec: &[f32],
        seed_k: usize,
        max_edges_per_seed: usize,
    ) -> (Vec<QueryResult>, String) {
        let seeds = self.search(query_vec, seed_k);
        let intro = "Knowledge graph retrieval: each section is a semantically matched node (seed) and its outgoing edges. \
relationship_probability is the confidence of the semantic link when the edge was created. \
token_cost is the graph traversal weight used for path ranking (higher means a weaker or more expensive link).";
        let mut parts: Vec<String> = vec![intro.to_string()];

        for seed in &seeds {
            let page = seed
                .metadata
                .get("page")
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let source = seed
                .metadata
                .get("source")
                .cloned()
                .unwrap_or_default();
            let id8 = seed
                .id
                .to_string()
                .chars()
                .take(8)
                .collect::<String>();

            let mut block = format!(
                "## Seed {id8} (semantic match score {:.4})\nSource: {source} | Page {page}\n{}\n",
                seed.score,
                truncate_for_prompt(&seed.source_text, 700),
            );

            let mut edges: Vec<&crate::graph::Edge> = self
                .graph
                .adj_list
                .get(&seed.id)
                .map(|v| v.iter().collect())
                .unwrap_or_default();
            edges.sort_by(|a, b| {
                b.relationship_probability
                    .partial_cmp(&a.relationship_probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.token.cmp(&b.token))
            });
            edges.truncate(max_edges_per_seed);

            if edges.is_empty() {
                block.push_str("Outgoing relationships: (none)\n");
            } else {
                block.push_str("Outgoing relationships:\n");
                for e in edges {
                    let neighbor = self.graph.nodes.get(&e.to);
                    let preview = neighbor
                        .map(|n| truncate_for_prompt(&n.source_text, 220))
                        .unwrap_or_else(|| "(missing node)".to_string());
                    let to_page = neighbor
                        .and_then(|n| n.metadata.get("page"))
                        .cloned()
                        .unwrap_or_else(|| "?".to_string());
                    let to8 = e
                        .to
                        .to_string()
                        .chars()
                        .take(8)
                        .collect::<String>();
                    block.push_str(&format!(
                        "- → node {to8} … page {to_page} | relationship_probability={:.4} | token_cost={} | linked text: {}\n",
                        e.relationship_probability,
                        e.token,
                        preview.replace('\n', " "),
                    ));
                }
            }
            parts.push(block);
        }

        let formatted = parts.join("\n");
        (seeds, formatted)
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