#![allow(dead_code)]

use std::collections::HashMap;

use crate::graph::structs::{DomainGraph, Edge, Node, NodeId};

fn truncate_for_prompt(s: &str, max_chars: usize) -> String {
    let n = s.chars().count();
    if n <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect::<String>() + "…"
}

fn node_matches_path_prefix(node: &Node, prefix: &str) -> bool {
    let p = prefix.trim().trim_end_matches('/').replace('\\', "/");
    if p.is_empty() {
        return true;
    }
    if let Some(path) = node.metadata.get("path") {
        let path = path.replace('\\', "/");
        if path == p || path.starts_with(&format!("{p}/")) {
            return true;
        }
    }
    node.source_uri.replace('\\', "/").contains(&p)
}

fn edge_preferred_for_prompt(a: &Edge, b: &Edge) -> std::cmp::Ordering {
    let la = a.label.eq_ignore_ascii_case("legacy");
    let lb = b.label.eq_ignore_ascii_case("legacy");
    match (la, lb) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b
            .relationship_probability
            .partial_cmp(&a.relationship_probability)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.token.cmp(&b.token)),
    }
}

fn pick_edge_for_same_target<'a>(a: &'a Edge, b: &'a Edge) -> &'a Edge {
    match edge_preferred_for_prompt(a, b) {
        std::cmp::Ordering::Less => b,
        _ => a,
    }
}

fn dedupe_out_edges_by_target(edges: Vec<&Edge>, max_edges: usize) -> Vec<&Edge> {
    let mut best_by_to: HashMap<NodeId, &Edge> = HashMap::new();
    for e in edges {
        best_by_to
            .entry(e.to)
            .and_modify(|cur| *cur = pick_edge_for_same_target(cur, e))
            .or_insert(e);
    }
    let mut deduped: Vec<&Edge> = best_by_to.into_values().collect();
    deduped.sort_by(|a, b| {
        b.relationship_probability
            .partial_cmp(&a.relationship_probability)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.token.cmp(&b.token))
    });
    deduped.truncate(max_edges);
    deduped
}

fn display_path(meta: &HashMap<String, String>) -> String {
    meta.get("path")
        .cloned()
        .or_else(|| meta.get("page").cloned())
        .unwrap_or_else(|| "?".to_string())
}

pub struct KnowledgeGraphQuery<'a> {
    graph: &'a DomainGraph,
}

impl<'a> KnowledgeGraphQuery<'a> {
    pub fn new(graph: &'a DomainGraph) -> Self {
        Self { graph }
    }

    /// Semantic seeds plus outgoing edges (probability, token cost, neighbor text) for LLM context.
    ///
    /// When `path_prefix` is set (e.g. GitHub module folder from the UI), seeds are preferentially drawn
    /// from nodes whose `metadata.path` is under that prefix; if none match, falls back to global search.
    pub fn search_with_relational_context(
        &self,
        query_vec: &[f32],
        seed_k: usize,
        max_edges_per_seed: usize,
        path_prefix: Option<&str>,
    ) -> (Vec<QueryResult>, String) {
        let trimmed = path_prefix.map(str::trim).filter(|s| !s.is_empty());

        let seeds = if let Some(pfx) = trimmed {
            let pool_k = seed_k.max(24).min(64);
            let pool = self.search(query_vec, pool_k);
            let filtered: Vec<QueryResult> = pool
                .into_iter()
                .filter(|r| {
                    self.graph
                        .nodes
                        .get(&r.id)
                        .map(|n| node_matches_path_prefix(n, pfx))
                        .unwrap_or(false)
                })
                .take(seed_k)
                .collect();
            if !filtered.is_empty() {
                filtered
            } else {
                self.search(query_vec, seed_k)
            }
        } else {
            self.search(query_vec, seed_k)
        };

        let mut intro = "Knowledge graph retrieval: each section is a semantically matched node (seed) and its outgoing edges. \
relationship_probability reflects how confident we are in that relationship. \
token_cost is an internal traversal weight for ranking paths (lower is typically cheaper/stronger for the ranker, not a dollar cost). \
Edge `label` is the relationship kind when known (e.g. imports, contains, defined_in, semantic_neighbor). \
The same node pair may appear with different labels (e.g. structure vs similarity); that is intentional. \
Do not treat token_cost or duplicate-looking rows as bugs unless the user explicitly asks about graph mechanics."
            .to_string();
        if let Some(pfx) = trimmed {
            intro.push_str(&format!(
                "\nScope: retrieval prefers nodes under repo path prefix `{pfx}`."
            ));
        }
        let mut parts: Vec<String> = vec![intro];

        for seed in &seeds {
            let page = display_path(&seed.metadata);
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

            let mut edges: Vec<&Edge> = self
                .graph
                .adj
                .get(&seed.id)
                .map(|v| v.iter().collect())
                .unwrap_or_default();
            edges.sort_by(|a, b| {
                b.relationship_probability
                    .partial_cmp(&a.relationship_probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.token.cmp(&b.token))
            });
            let edges = dedupe_out_edges_by_target(edges, max_edges_per_seed);

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
                        .map(|n| display_path(&n.metadata))
                        .unwrap_or_else(|| "?".to_string());
                    let to8 = e
                        .to
                        .to_string()
                        .chars()
                        .take(8)
                        .collect::<String>();
                    block.push_str(&format!(
                        "- label={} → node {to8} … page {to_page} | relationship_probability={:.4} | token_cost={} | linked text: {}\n",
                        e.label,
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

    /// Traverse from a node, return ordered context window for LLM (dual-weight shortest path).
    pub fn traverse_context(&self, start: NodeId, goal: NodeId) -> Option<Vec<QueryResult>> {
        self.graph.shorted_path(start, goal).map(|path| {
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
    pub fn neighborhood(&self, start: NodeId, depth: usize) -> Vec<QueryResult> {
        let all = self.graph.bfs_internal(start);
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
