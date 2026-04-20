mod graph;
mod ingestion;
mod ingestion_registry;
mod processing;
mod query;
mod server;

use anyhow::Context;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use graph::{
    graph_registry::GraphRegistry,
    structs::{DomainGraph, Edge, EdgeId, GraphId, Node, NodeId},
    enums::{Domain, GraphQuery, GraphResult, NodeKind, NodePredicate},
    fluvio_graph::FluvioGraph,
    EmbeddingContext,
};
use std::collections::HashMap;

// ── Constants ─────────────────────────────────────────────────────────────────

const GRAPHS_DIR: &str = "fluvio_graphs";   // one JSON per domain graph + meta.json
const DEFAULT_MODEL: &str = "claude-sonnet-4-5";

fn anthropic_api_key() -> anyhow::Result<String> {
    std::env::var("ANTHROPIC_API_KEY").context(
        "ANTHROPIC_API_KEY not set — add it to `.env` (ANTHROPIC_API_KEY=...) or export ANTHROPIC_API_KEY=sk-ant-...",
    )
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load `ANTHROPIC_API_KEY` (and other vars) from `.env` in the current working directory when present.
    let _ = dotenvy::dotenv();

    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("ingest") => {
            // fluvio ingest <domain> <file>
            // domain: pdf | email | whatsapp | music | codebase | ...
            let domain_str = args.get(2).context("usage: fluvio ingest <domain> <file>")?;
            let path       = args.get(3).context("usage: fluvio ingest <domain> <file>")?;
            cmd_ingest(domain_str, path).await?;
        }
        Some("chat") => {
            // fluvio chat <domain>   (defaults to "pdf" if omitted)
            let domain_str = args.get(2).map(|s| s.as_str()).unwrap_or("pdf");
            cmd_chat(domain_str, &anthropic_api_key()?).await?;
        }
        Some("stats") => {
            cmd_stats()?;
        }
        Some("server") => {
            server::serve(anthropic_api_key()?).await?;
        }
        Some("ingest-email") => {
            cmd_ingest_email().await?;
        }
        _ => {
            println!("Fluvio KG — multi-graph CLI");
            println!("  fluvio ingest <domain> <file>   ingest file into domain graph");
            println!("  fluvio chat   [domain]           chat with a domain graph");
            println!("  fluvio stats                     show all graph stats");
            println!("  fluvio server                    start REST API on :8001");
            println!("  fluvio ingest-email               ingest email into email graph");
            println!();
            println!("  domains: pdf | email | whatsapp | music | codebase");
        }
    }
    Ok(())
}

// ── Registry bootstrap ────────────────────────────────────────────────────────

/// Build a fresh registry and load any existing graphs from disk.
/// Every domain graph that has a saved JSON snapshot is loaded automatically.
fn bootstrap_registry(embed_ctx: Arc<Mutex<EmbeddingContext>>) -> anyhow::Result<GraphRegistry> {
    let _ = embed_ctx;
    let mut registry = GraphRegistry::new();

    // Known domains — we always register them so agents can write to them
    // even if they have no data yet.
    let domains = [
        (GraphId::new("pdf"),       Domain::Pdf),
        (GraphId::new("email"),     Domain::Email),
        (GraphId::new("whatsapp"),  Domain::Whatsapp),
        (GraphId::new("music"),     Domain::Music),
        (GraphId::new("codebase"),  Domain::Codebase),
    ];

    for (id, domain) in domains {
        let mut g = DomainGraph::new(id.clone(), domain);
        let path  = format!("{}/{}.json", GRAPHS_DIR, id.0);
        if std::path::Path::new(&path).exists() {
            println!("Loading graph '{}' from {}", id.0, path);
            g.load(&path)?;
        }
        registry.register(g);
    }

    // Load meta-graph if it exists.
    let meta_path = format!("{}/meta.json", GRAPHS_DIR);
    if std::path::Path::new(&meta_path).exists() {
        println!("Loading meta-graph from {meta_path}");
        registry.meta_mut().load(&meta_path)?;
    }

    Ok(registry)
}

fn domain_from_str(s: &str) -> anyhow::Result<(GraphId, Domain)> {
    match s {
        "pdf"      => Ok((GraphId::new("pdf"),       Domain::Pdf)),
        "email"    => Ok((GraphId::new("email"),     Domain::Email)),
        "whatsapp" => Ok((GraphId::new("whatsapp"),  Domain::Whatsapp)),
        "music"    => Ok((GraphId::new("music"),     Domain::Music)),
        "codebase" => Ok((GraphId::new("codebase"),  Domain::Codebase)),
        other      => Ok((
            GraphId::new(other),
            Domain::Custom(other.to_string()),
        )),
    }
}

// ── ingest ────────────────────────────────────────────────────────────────────

async fn cmd_ingest(domain_str: &str, file_path: &str) -> anyhow::Result<()> {
    println!("Ingesting [{domain_str}]: {file_path}");

    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let mut registry = bootstrap_registry(embed_ctx.clone())?;
    let (graph_id, _domain) = domain_from_str(domain_str)?;

    // Pull the target graph out of the registry, ingest into it, put it back.
    // We do this by getting a mutable reference — registry owns it the whole time.
    {
        let graph = registry
            .get_mut(&graph_id)
            .context(format!("Graph '{}' not registered", graph_id.0))?;

        // Source-specific chunking — each domain knows how to read its own format.
        // Right now only PDF is implemented; others will plug in here as you add connectors.
        let chunks: Vec<(String, usize)> = match domain_str {
            "pdf" => {
                use processing::mmap_manager::PDFChunkIterator;
                let iter = PDFChunkIterator::new(file_path, 1)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                iter.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| anyhow::anyhow!("{e}"))?
                    .into_iter()
                    .enumerate()
                    .map(|(i, text)| (text, i + 1))
                    .collect()
            }
            // Future connectors slot in here:
            // "email"    => email::parse(file_path)?,
            // "whatsapp" => whatsapp::parse(file_path)?,
            // "music"    => music::transcribe(file_path)?,
            _ => anyhow::bail!("No ingestion connector for domain '{domain_str}' yet"),
        };

        println!("Found {} chunks", chunks.len());

        for (chunk, page_num) in &chunks {
            if chunk.trim().is_empty() { continue; }
            print!("  Chunk {page_num}/{} ... ", chunks.len());
            io::stdout().flush()?;
            let id = ingest_chunk(graph, &embed_ctx, chunk, domain_str, *page_num)?;
            println!("{}", &id.0.to_string()[..8]);
        }

        println!("Wiring edges by similarity...");
        wire_edges(graph, 0.35)?;

        let edge_count = graph.edge_count();
        println!("  {} edges wired", edge_count);
    }

    // Persist everything.
    registry.save_all(GRAPHS_DIR)?;
    println!("Saved to {GRAPHS_DIR}/");

    Ok(())
}

// ── chat ──────────────────────────────────────────────────────────────────────

async fn cmd_chat(domain_str: &str, api_key: &str) -> anyhow::Result<()> {
    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let registry  = bootstrap_registry(embed_ctx.clone())?;
    let (graph_id, _) = domain_from_str(domain_str)?;

    let graph = registry
        .get(&graph_id)
        .context(format!("Graph '{}' not found — ingest something first", graph_id.0))?;

    if graph.node_count() == 0 {
        println!("Graph '{}' is empty. Run `fluvio ingest {domain_str} <file>` first.", graph_id.0);
        return Ok(());
    }

    println!(
        "Graph '{}' loaded: {} nodes, {} edges",
        graph_id.0,
        graph.node_count(),
        graph.edge_count(),
    );
    println!("Chat with your [{domain_str}] graph. Type 'exit' to quit.\n");

    let client = reqwest::Client::new();
    let mut history: Vec<serde_json::Value> = Vec::new();

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut question = String::new();
        io::stdin().read_line(&mut question)?;
        let question = question.trim();
        if question == "exit" || question.is_empty() { break; }

        // 1. Embed question
        let query_vec = embed_ctx.lock().unwrap().embed(question)?;

        // 2. Retrieve context from this domain graph
        let results = search_graph(graph, &query_vec, 6);

        if results.is_empty() {
            println!("Assistant: No relevant content found in the [{domain_str}] graph.\n");
            continue;
        }

        // 3. Build context: seeds + outgoing edges (probability, token, labels)
        let context = format_fluvio_chat_context(graph, &results, 5);

        // 4. Call Claude
        history.push(serde_json::json!({"role": "user", "content": question}));

        let system = format!(
            "You are a helpful assistant answering questions using the user's [{domain_str}] knowledge graph.\n\
             The context lists semantically retrieved seed nodes and their outgoing edges (relationship_probability, token_cost, edge_label, linked neighbor text).\n\
             Answer using ONLY this graph context. Be concise and direct.\n\
             If the answer is not supported, say \"I don't see that in the knowledge graph context.\"\n\n\
             KNOWLEDGE GRAPH CONTEXT:\n{context}"
        );

        let res = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": DEFAULT_MODEL,
                "max_tokens": 1024,
                "system": system,
                "messages": history,
            }))
            .send().await?
            .json::<serde_json::Value>().await?;

        let answer = res["content"][0]["text"]
            .as_str()
            .unwrap_or("(no response)")
            .to_string();

        println!("\nAssistant: {answer}");
        println!("\nSources:");
        for r in &results {
            let page   = r.metadata.get("page").cloned().unwrap_or_else(|| "?".into());
            let source = r.metadata.get("source").cloned().unwrap_or_default();
            println!("  [{:.2}] {source} page {page}", r.score);
        }
        println!();

        history.push(serde_json::json!({"role": "assistant", "content": answer}));
    }

    Ok(())
}

fn ingest_chunk(
    graph: &mut (dyn FluvioGraph + Send + Sync),
    embed_ctx: &Arc<Mutex<EmbeddingContext>>,
    text: &str,
    source_hint: &str,
    page: usize,
) -> anyhow::Result<NodeId> {
    let embeddings = embed_ctx.lock().unwrap().embed(text)?;
    let node = Node {
        id: NodeId::from_content(source_hint, &format!("{}::{}", page, text)),
        domain: graph.domain().clone(),
        source_uri: source_hint.to_string(),
        source_text: text.to_string(),
        embeddings,
        metadata: HashMap::from([
            ("source".to_string(), source_hint.to_string()),
            ("page".to_string(), page.to_string()),
        ]),
        kind: NodeKind::Artifcat,
    };
    Ok(graph.insert_node(node)?)
}

fn wire_edges(graph: &mut (dyn FluvioGraph + Send + Sync), threshold: f32) -> anyhow::Result<()> {
    let node_ids: Vec<NodeId> = match graph.query(GraphQuery::Filter(
        NodePredicate::ByDomain(graph.domain().clone()),
    )) {
        GraphResult::Nodes(nodes) => nodes.into_iter().map(|n| n.id).collect(),
        _ => Vec::new(),
    };

    for id in &node_ids {
        let query_vec = match graph.get_node(id) {
            Some(node) => node.embeddings.clone(),
            None => continue,
        };

        let neighbors = match graph.query(GraphQuery::SimilarTo {
            embedding: query_vec,
            top_k: 6,
        }) {
            GraphResult::Scored(scores) => scores,
            _ => Vec::new(),
        };

        for (neighbor_id, sim) in neighbors {
            if neighbor_id == *id || sim < threshold {
                continue;
            }

            let already = graph
                .get_edges_from(id)
                .iter()
                .any(|edge| edge.to == neighbor_id);
            if already {
                continue;
            }

            let token_cost = ((1.0 - sim) * 10_000.0) as i32;
            let edge = Edge {
                id: EdgeId::new(),
                from: *id,
                to: neighbor_id,
                token: token_cost,
                relationship_probability: sim as f64,
                label: "semantic_similarity".to_string(),
                metadata: HashMap::new(),
            };
            let _ = graph.insert_edge(edge)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct SearchResult {
    id: NodeId,
    score: f32,
    source_text: String,
    metadata: HashMap<String, String>,
}

fn truncate_for_prompt(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect::<String>() + "…"
}

/// Seeds from similarity search plus outgoing edges (probability, token, label, neighbor text).
fn format_fluvio_chat_context(
    graph: &(dyn FluvioGraph + Send + Sync),
    seeds: &[SearchResult],
    max_edges_per_seed: usize,
) -> String {
    let intro = "Knowledge graph retrieval: each section is a semantically matched node (seed) and its outgoing edges. \
relationship_probability is the confidence of the semantic link when the edge was created. \
token_cost is the graph traversal weight used for path ranking (higher means a weaker or more expensive link).";
    let mut parts: Vec<String> = vec![intro.to_string()];

    for seed in seeds {
        let page = seed
            .metadata
            .get("page")
            .cloned()
            .unwrap_or_else(|| "?".into());
        let source = seed
            .metadata
            .get("source")
            .cloned()
            .unwrap_or_default();
        let id8 = seed.id.0.to_string().chars().take(8).collect::<String>();

        let mut block = format!(
            "## Seed {id8} (semantic match score {:.4})\nSource: {source} | Page {page}\n{}\n",
            seed.score,
            truncate_for_prompt(&seed.source_text, 700),
        );

        let mut edges: Vec<_> = graph.get_edges_from(&seed.id).to_vec();
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
            for e in &edges {
                let neighbor = graph.get_node(&e.to);
                let preview = neighbor
                    .map(|n| truncate_for_prompt(&n.source_text, 220))
                    .unwrap_or_else(|| "(missing node)".into());
                let to_page = neighbor
                    .and_then(|n| n.metadata.get("page"))
                    .cloned()
                    .unwrap_or_else(|| "?".into());
                let to8 = e.to.0.to_string().chars().take(8).collect::<String>();
                block.push_str(&format!(
                    "- → node {to8} … page {to_page} | relationship_probability={:.4} | token_cost={} | edge_label={} | linked text: {}\n",
                    e.relationship_probability,
                    e.token,
                    e.label,
                    preview.replace('\n', " "),
                ));
            }
        }
        parts.push(block);
    }

    parts.join("\n")
}

fn search_graph(
    graph: &(dyn FluvioGraph + Send + Sync),
    query_vec: &[f32],
    top_k: usize,
) -> Vec<SearchResult> {
    let scored = match graph.query(GraphQuery::SimilarTo {
        embedding: query_vec.to_vec(),
        top_k,
    }) {
        GraphResult::Scored(scores) => scores,
        _ => Vec::new(),
    };

    scored
        .into_iter()
        .filter_map(|(id, score)| {
            graph.get_node(&id).map(|node| SearchResult {
                id,
                score,
                source_text: node.source_text.clone(),
                metadata: node.metadata.clone(),
            })
        })
        .collect()
}

// ── stats ─────────────────────────────────────────────────────────────────────

fn cmd_stats() -> anyhow::Result<()> {
    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let registry  = bootstrap_registry(embed_ctx)?;

    println!("Fluvio Graph Registry — {GRAPHS_DIR}/");
    println!();

    // Domain graphs
    for id in ["pdf", "email", "whatsapp", "music", "codebase"] {
        let gid = GraphId::new(id);
        if let Some(graph) = registry.get(&gid) {
            println!(
                "  [{id:>10}]  nodes: {:>5}   edges: {:>5}",
                graph.node_count(),
                graph.edge_count(),
            );
        }
    }

    // Meta graph
    let meta = registry.meta();
    println!(
        "  [{:>10}]  nodes: {:>5}   edges: {:>5}",
        "meta",
        meta.node_count(),
        meta.edge_count(),
    );

    Ok(())
}

// ── ingest-email ─────────────────────────────────────────────────────────────
async fn cmd_ingest_email() -> anyhow::Result<()> {
    println!("Ingesting email...");
    Ok(())
}