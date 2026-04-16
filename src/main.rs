mod graph;
mod ingestion;
mod processing;
mod query;
mod server;

use anyhow::Context;
use std::sync::{Arc, Mutex};
use std::io::{self, Write};
use graph::{EmbeddingContext, Graph};
use ingestion::IngestionPipeline;
use query::KnowledgeGraphQuery;

const GRAPH_PATH: &str = "fluvio_graph.json";

fn anthropic_api_key() -> anyhow::Result<String> {
    std::env::var("ANTHROPIC_API_KEY").context(
        "ANTHROPIC_API_KEY is not set; export it for ingest and chat (e.g. export ANTHROPIC_API_KEY=sk-ant-...)",
    )
}

const DEFAULT_MODEL: &str = "claude-sonnet-4-5"; // swap for gpt-4o, gemini etc later


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("ingest") => {
            let path = args
                .get(2)
                .context("usage: fluvio ingest <file.pdf>")?;
            cmd_ingest(path).await?;
        }
        Some("chat") => {
            cmd_chat(&anthropic_api_key()?).await?;
        }
        Some("stats") => {
            cmd_stats()?;
        }
        Some("server") => {
            server::serve(anthropic_api_key()?).await?;
        }
        _ => {
            println!("Fluvio KG — local CLI");
            println!("  fluvio ingest <file.pdf>   ingest PDF into graph");
            println!("  fluvio chat                chat with your graph");
            println!("  fluvio stats               show graph stats");
            println!("  fluvio serve               start REST API on :8001");
        }

    }
    Ok(())
}

// ── ingest ────────────────────────────────────────────────────────────────────
async fn cmd_ingest(pdf_path: &str) -> anyhow::Result<()> {
    println!("Ingesting: {pdf_path}");

    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let mut graph  = Graph::new(embed_ctx);

    // Load existing graph if present so ingests are additive
    if std::path::Path::new(GRAPH_PATH).exists() {
        println!("Loading existing graph from {GRAPH_PATH}");
        graph.load(GRAPH_PATH)?;
    }

    let mut pipeline = IngestionPipeline::new(graph);

    // PDFChunkIterator mmap-opens the path and yields one string per `pages_per_chunk` page(s).
    let chunks: Vec<String> = {
        use processing::mmap_manager::PDFChunkIterator;
        let iter = PDFChunkIterator::new(pdf_path, 1)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        iter.collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("{e}"))?
    };

    println!("Found {} pages/chunks", chunks.len());

    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.trim().is_empty() { continue; }

        print!("  Embedding Chunk {}/{} ... ", i + 1, chunks.len());
        io::stdout().flush()?;
        let id = pipeline.ingest_chunk(chunk, "pdf", i + 1)?;
        println!("{} nodes", &id.to_string()[..8]);
    }

    println!("Wiring edges by similarity ....");
    io::stdout().flush()?;
    pipeline.wire_edges(0.35);

    let total_edges: usize = pipeline.graph.adj_list.values().map(|e| e.len()).sum();

    println!("Done");

    pipeline.graph.save(GRAPH_PATH)?;

    println!("\nDone.");
    println!("  Edges : {total_edges}");
    println!("  Saved : {GRAPH_PATH}");

    Ok(())
}

// ── chat ──────────────────────────────────────────────────────────────────────
async fn cmd_chat(api_key: &str) -> anyhow::Result<()> {
    if !std::path::Path::new(GRAPH_PATH).exists() {
        println!("No graph found. Run `fluvio ingest <file.pdf>` first.");
        return Ok(());
    }

    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let mut graph  = Graph::new(embed_ctx);
    graph.load(GRAPH_PATH)?;

    println!("Graph loaded: {} nodes, {} edges",
        graph.nodes.len(),
        graph.adj_list.values().map(|e| e.len()).sum::<usize>()
    );
    println!("Chat with your document. Type 'exit' to quit.\n");


    let client   = reqwest::Client::new();
    let mut history: Vec<serde_json::Value> = Vec::new();

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut question = String::new();
        io::stdin().read_line(&mut question)?;
        let question = question.trim();

        if question == "exit" || question.is_empty() { break; }

        // 1. Embed question
        let query_vec = graph.embed_ctx.lock().unwrap().embed(question)?;

        // 2. Retrieve context from graph
        let kg      = KnowledgeGraphQuery::new(&graph);
        let results = kg.search(&query_vec, 6);

        if results.is_empty() {
            println!("Assistant: I couldn't find relevant content in the graph.\n");
            continue;
        }

        // 3. Build context block
        let context = results.iter()
            .map(|r| {
                let page = r.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string());
                let source = r.metadata.get("source").cloned().unwrap_or_default();
                format!("[{source} | page {page}]\n{}", r.source_text)
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        // 4. Build message history
        history.push(serde_json::json!({"role": "user", "content": question}));

        let system = format!(
            "You are a helpful assistant answering questions about the user's documents.\n\
             Answer using ONLY the context below. Be concise and direct.\n\
             If the answer is not in the context, say \"I don't see that in the document.\"\n\n\
             CONTEXT:\n{context}"
        );

        // 5. Call Claude
        let res = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-sonnet-4-5",
                "max_tokens": 1024,
                "system": system,
                "messages": history
            }))
            .send().await?
            .json::<serde_json::Value>().await?;

        eprintln!("DEbug raw response: {}", serde_json::to_string_pretty(&res)?);

        let answer = res["content"][0]["text"]
            .as_str()
            .unwrap_or("(no response)")
            .to_string();

        // 6. Print answer + sources
        println!("\nAssistant: {answer}");
        println!("\nSources:");
        for r in &results {
            let page = r.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string());
            let source = r.metadata.get("source").cloned().unwrap_or_default();
            println!("  [{:.2}] {source} page {page}", r.score);
        }
        println!();

        // 7. Append assistant turn to history for next question
        history.push(serde_json::json!({"role": "assistant", "content": answer}));
    }

    Ok(())
}

// ── stats ─────────────────────────────────────────────────────────────────────
fn cmd_stats() -> anyhow::Result<()> {
    if !std::path::Path::new(GRAPH_PATH).exists() {
        println!("No graph found. Run `fluvio ingest <file.pdf>` first.");
        return Ok(());
    }

    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let mut graph  = Graph::new(embed_ctx);
    graph.load(GRAPH_PATH)?;

    let total_edges: usize = graph.adj_list.values().map(|e| e.len()).sum();
    println!("Graph: {GRAPH_PATH}");
    println!("  Nodes : {}", graph.nodes.len());
    println!("  Edges : {total_edges}");

    // Show top 10 most connected nodes
    let mut degrees: Vec<(&uuid::Uuid, usize)> = graph.adj_list
        .iter()
        .map(|(id, edges)| (id, edges.len()))
        .collect();
    degrees.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\nTop nodes by connections:");
    for (id, deg) in degrees.iter().take(10) {
        let name = graph.nodes.get(id)
            .and_then(|n| n.metadata.get("name"))
            .cloned()
            .unwrap_or_else(|| id.to_string());
        println!("  {deg:>3} edges  {name}");
    }

    Ok(())
}