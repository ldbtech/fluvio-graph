//! Knowledge context assembly for LLM prompts.
//!
//! Ported from the monolith's `build_knowledge_from_surreal` and
//! `knowledge_context_line_surreal` in `src/routes/twin.rs`.

use std::collections::HashSet;
use crate::graph::GraphNode;

pub const SIM_TOP_K:              usize = 36;
pub const SEED_K:                 usize = 6;
pub const BFS_DEPTH:              usize = 2;
pub const NEIGHBOR_CAP:           usize = 10;
pub const SEED_TEXT_CHARS:        usize = 700;
pub const NEIGHBOR_PREVIEW_CHARS: usize = 220;
pub const MAX_CONTEXT_CHARS:      usize = 28_000;

// ── Context line formatting ───────────────────────────────────────────────────

pub fn node_context_line(node: &GraphNode, max_chars: usize) -> String {
    let kind = node.metadata.iter()
        .find(|(k, _)| k == "kind")
        .map(|(_, v)| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("chunk");

    let tag = node.metadata.iter()
        .find(|(k, _)| k == "filename" || k == "title")
        .map(|(_, v)| format!(" `{}`", v.replace('`', "'")))
        .unwrap_or_default();

    let excerpt: String = node.source_text.chars().take(max_chars).collect();
    format!("- [{kind}]{tag}\n  {excerpt}")
}

pub fn display_path(node: &GraphNode) -> String {
    node.metadata.iter()
        .find(|(k, _)| k == "path" || k == "page")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "?".to_string())
}

// ── Context assembly ─────────────────────────────────────────────────────────

/// Build the full knowledge context string from seeds + BFS neighbors.
/// This is what gets injected into the Claude system prompt.
pub fn build_context_from_seeds(
    seeds:     &[(GraphNode, Vec<GraphNode>)],  // (seed, neighbors)
    user_name: &str,
) -> (String, Vec<ContextSource>) {
    let mut parts:   Vec<String>        = Vec::new();
    let mut sources: Vec<ContextSource> = Vec::new();
    let mut seen:    HashSet<String>    = HashSet::new();

    let intro = format!(
        "Knowledge graph retrieval for {}. Each section is a semantically matched \
         chunk and its graph neighborhood (nodes linked in the graph).\n\
         Answer using ONLY this context. Be concise and grounded.\n",
        user_name
    );
    parts.push(intro);

    for (rank, (seed, neighbors)) in seeds.iter().enumerate() {
        let id_short: String = seed.id.chars().take(8).collect();
        let page = display_path(seed);

        let block_head = format!(
            "## Seed {} (rank {}) | page {} | domain {}\n{}\n",
            id_short,
            rank + 1,
            page,
            seed.domain,
            truncate(&seed.source_text, SEED_TEXT_CHARS),
        );

        seen.insert(seed.id.clone());

        let mut neighbor_lines: Vec<String> = Vec::new();
        for n in neighbors {
            if seen.contains(&n.id) || neighbor_lines.len() >= NEIGHBOR_CAP {
                continue;
            }
            seen.insert(n.id.clone());
            let p     = display_path(n);
            let short: String = n.id.chars().take(8).collect();
            neighbor_lines.push(format!(
                "- neighbor {} … page {} | {}",
                short,
                p,
                truncate(&n.source_text, NEIGHBOR_PREVIEW_CHARS).replace('\n', " "),
            ));
        }

        let neigh_block = if neighbor_lines.is_empty() {
            "Graph neighborhood: (no linked nodes within hop limit)\n".to_string()
        } else {
            format!("Graph neighborhood (hops ≤{BFS_DEPTH}):\n{}\n", neighbor_lines.join("\n"))
        };

        parts.push(format!("{block_head}{neigh_block}"));

        sources.push(ContextSource {
            id:    seed.id.clone(),
            page,
            score: seed.score,
            text:  seed.source_text.chars().take(120).collect(),
        });
    }

    let mut context = parts.join("\n");

    // Cap total context size
    if context.len() > MAX_CONTEXT_CHARS {
        context.truncate(MAX_CONTEXT_CHARS);
        context.push_str("\n\n[context truncated]");
    }

    (context, sources)
}

/// Build the system prompt for Claude.
pub fn build_system_prompt(
    context:   &str,
    user_name: &str,
    is_self:   bool,
) -> String {
    if is_self {
        format!(
            "You are {user_name}'s AI twin. You speak in first person as {user_name}.\n\
             Answer questions naturally and conversationally as if you are them.\n\
             Ground your answers in the KNOWLEDGE CONTEXT below.\n\
             If something is not in the context, say so honestly in their voice.\n\n\
             KNOWLEDGE CONTEXT:\n{context}"
        )
    } else {
        format!(
            "You are a helpful assistant answering questions about a person's knowledge graph.\n\
             Answer using ONLY the KNOWLEDGE CONTEXT below. Be concise.\n\
             If the answer is not supported by the context, say so clearly.\n\n\
             KNOWLEDGE CONTEXT:\n{context}"
        )
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ContextSource {
    pub id:    String,
    pub page:  String,
    pub score: f32,
    pub text:  String,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}