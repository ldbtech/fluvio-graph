use axum::{
    Json, Router, extract::{Multipart, State}, http::Method, routing::{get, post}
};
use axum::http::StatusCode;
use tower_http::cors::{Any, CorsLayer};
use std::sync::{Arc, Mutex};
use crate::{
    graph::{EmbeddingContext, Graph},
    ingestion::IngestionPipeline,
    query::KnowledgeGraphQuery,
    processing::mmap_manager::PDFChunkIterator,
};

const GRAPH_PATH: &str = "fluvio_graph.json";

#[derive(Clone)]
pub struct AppState {
    pub pipeline: Arc<Mutex<IngestionPipeline>>,
    pub api_key: String,
}

pub async fn serve(api_key: String) -> anyhow::Result<()> {
    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let mut graph = Graph::new(embed_ctx);

    if std::path::Path::new(GRAPH_PATH).exists() {
        println!("Loading existing graph from {GRAPH_PATH}");
        graph.load(GRAPH_PATH)?;
    }

    let pipeline = Arc::new(Mutex::new(IngestionPipeline::new(graph)));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let state = AppState { pipeline, api_key };

    let app = Router::new()
        .route("/ingest/pdf", post(ingest_pdf))
        .route("/graph", get(get_graph))
        .route("/chat", post(chat))
        .layer(cors)
        .with_state(state);

    let listner = tokio::net::TcpListener::bind("0.0.0.0:8001").await?;

    println!("KG-GRAPH Listening on http://localhost:8001");

    axum::serve(listner, app).await?;

    Ok(())
}

// ---- POST /ingest/pdf
#[derive(serde::Serialize)]
struct IngestResponse {
    nodes: usize,
    edges: usize,
}

async fn ingest_pdf(
    State(state): State<AppState>,
    mut multipart: Multipart
) -> Result<Json<IngestResponse>, (axum::http::StatusCode, String)>{
    let field = multipart.next_field().await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
        .ok_or((axum::http::StatusCode::BAD_REQUEST, "no file".to_string()))?;

    let bytes = field.bytes().await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let tmp_path = "/tmp/fluvio_upload.pdf";
    std::fs::write(tmp_path, &bytes)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;


    // Chunk + embed
    let chunks = PDFChunkIterator::new(tmp_path, 1)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut pipeline = state.pipeline.lock().unwrap();

    for (i, chunk) in chunks.iter().enumerate() {
        if chunk.trim().is_empty() { continue; }
        let _ = pipeline.ingest_chunk(chunk, "pdf", i + 1);
    }

    pipeline.wire_edges(0.35);
    pipeline.graph.save(GRAPH_PATH)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_edges: usize = pipeline.graph.adj_list.values().map(|e| e.len()).sum();

    Ok(Json(IngestResponse { nodes: pipeline.graph.nodes.len(), edges: total_edges }))

}

// ---- Get /graph
#[derive(serde::Serialize)]
struct GraphResponse {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[derive(serde::Serialize)]
struct GraphNode {
    id: String,
    label: String,
    page: String,
    source: String,
}

#[derive(serde::Serialize)]
struct GraphEdge {
    from: String,
    to: String,
    token: i32, 
    probability: f64,
}

async fn get_graph(State(state): State<AppState>) -> Json<GraphResponse> {
    let pipeline = state.pipeline.lock().unwrap();

    let nodes = pipeline.graph.nodes.values().map(|n| {
        let label: String = n.source_text.chars().take(60).collect();

        GraphNode {
            id:         n.id.to_string(),
            label,
            page:       n.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string()),
            source:     n.metadata.get("source").cloned().unwrap_or_default(),   
        }
    }).collect();

    let edges = pipeline.graph.adj_list.values().flatten().map(|e| {
        GraphEdge {
            from:           e.from.to_string(),
            to:             e.to.to_string(),
            token:          e.token,
            probability:    e.relationship_probability, 
        }
    }).collect();

    Json(GraphResponse { nodes: nodes, edges: edges })
}


// POST --- /chat
#[derive(serde::Deserialize)]
struct ChatRequest {
    question: String,
    history: Vec<HistoryMessage>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct HistoryMessage {
    role:    String,
    content: String,
}

#[derive(serde::Serialize)]
struct ChatResponse {
    answer:  String,
    sources: Vec<SourceNode>,
}

#[derive(serde::Serialize)]
struct SourceNode {
    id:    String,
    page:  String,
    score: f32,
    text:  String,
}

#[axum::debug_handler]
async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {

    // ── do all graph work under the lock, then drop it ────────────────────────
    let (query_vec, context, sources) = {
        let pipeline = state.pipeline.lock().unwrap();

        let query_vec = pipeline.graph.embed_ctx
            .lock().unwrap()
            .embed(&req.question)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let kg = KnowledgeGraphQuery::new(&pipeline.graph);
        let results = kg.search(&query_vec, 6);

        if results.is_empty() {
            return Ok(Json(ChatResponse {
                answer: "I could not find relevant content in the graph.".to_string(),
                sources: vec![],
            }));
        }

        let context = results.iter().map(|r| {
            let page = r.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string());
            format!("[page {page}]\n{}", r.source_text)
        }).collect::<Vec<_>>().join("\n\n---\n\n");

        let sources: Vec<SourceNode> = results.iter().map(|r| SourceNode {
            id:    "".to_string(),
            page:  r.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string()),
            score: r.score,
            text:  r.source_text.chars().take(120).collect(),
        }).collect();

        (query_vec, context, sources)
    }; // ← lock drops here, before any await

    // ── everything below is lock-free ─────────────────────────────────────────
    let mut messages: Vec<serde_json::Value> = req.history.iter().map(|h| {
        serde_json::json!({"role": h.role, "content": h.content})
    }).collect();
    messages.push(serde_json::json!({"role": "user", "content": req.question}));

    let system = format!(
        "You are a helpful assistant answering questions about the user's documents.\n\
         Answer using ONLY the context below. Be concise.\n\
         If the answer is not in the context, say \"I don't see that in the document.\"\n\n\
         CONTEXT:\n{context}"
    );

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &state.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 1024,
            "system": system,
            "messages": messages
        }))
        .send().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .json::<serde_json::Value>().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let answer = res["content"][0]["text"]
        .as_str()
        .unwrap_or("(no response)")
        .to_string();

    Ok(Json(ChatResponse { answer, sources }))
}