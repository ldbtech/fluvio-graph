use axum::{
    Json, Router,
    extract::{Multipart, Query, State},
    http::Method,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use crate::{
    graph::{EmbeddingContext, Graph},
    ingestion::IngestionPipeline,
    ingestion_registry::{
        connector::FluvioConnector,
        email::{
            auth::{exchange_code, get_auth_url},
            connector::GmailConnector,
            credentials_exist,
            GmailSyncProgress, GmailSyncProgressSnapshot, GmailSyncResultSummary,
        },
    },
    ingestion_registry::documents::pdf::PDFChunkIterator,
    query::KnowledgeGraphQuery,
};

/// Workspace graphs live under `fluvio_graphs/workspace/` (`unified.json` plus filtered snapshots).
const WORKSPACE_GRAPHS_DIR: &str = "fluvio_graphs/workspace";
const WORKSPACE_UNIFIED: &str = "fluvio_graphs/workspace/unified.json";
const WORKSPACE_PDF: &str = "fluvio_graphs/workspace/pdf.json";
const WORKSPACE_EMAIL: &str = "fluvio_graphs/workspace/email.json";
const LEGACY_GRAPH_PATH: &str = "fluvio_graph.json";
const WORKSPACE_PROJECTS_DIR: &str = "fluvio_graphs/projects";

fn sanitize_project_id(raw: &str) -> Result<String, (StatusCode, String)> {
    let s = raw.trim();
    if s.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "project id is empty".to_string()));
    }
    if s.len() > 64 {
        return Err((StatusCode::BAD_REQUEST, "project id must be at most 64 characters".to_string()));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "project id may only contain letters, digits, hyphen, and underscore".to_string(),
        ));
    }
    Ok(s.to_string())
}

fn persist_workspace_snapshots(graph: &Graph) -> anyhow::Result<()> {
    std::fs::create_dir_all(WORKSPACE_GRAPHS_DIR)?;
    graph.save(WORKSPACE_UNIFIED)?;
    graph.save_filtered(WORKSPACE_PDF, |n| {
        n.metadata.get("source").map(|s| s == "pdf").unwrap_or(false)
    })?;
    graph.save_filtered(WORKSPACE_EMAIL, |n| {
        n.metadata
            .get("source")
            .map(|s| s == "email" || s == "gmail")
            .unwrap_or(false)
    })?;
    Ok(())
}

fn load_server_graph(graph: &mut Graph) -> anyhow::Result<()> {
    if std::path::Path::new(WORKSPACE_UNIFIED).exists() {
        println!("Loading existing graph from {WORKSPACE_UNIFIED}");
        graph.load(WORKSPACE_UNIFIED)?;
        return Ok(());
    }
    if std::path::Path::new(LEGACY_GRAPH_PATH).exists() {
        println!(
            "Loading existing graph from {LEGACY_GRAPH_PATH} (migrating snapshots to {WORKSPACE_GRAPHS_DIR}/)"
        );
        graph.load(LEGACY_GRAPH_PATH)?;
        persist_workspace_snapshots(graph)?;
        return Ok(());
    }
    println!("No workspace graph yet; new data will be saved under {WORKSPACE_GRAPHS_DIR}/");
    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub pipeline: Arc<Mutex<IngestionPipeline>>,
    pub api_key: String,
    /// Expected `state` query param on `/connect/gmail/callback` (CSRF).
    pub oauth_csrf: Arc<Mutex<Option<String>>>,
    /// Gmail sync progress for `GET /sync/gmail/progress` while `POST /sync/gmail` runs in the background.
    pub gmail_progress: Arc<GmailSyncProgress>,
}

pub async fn serve(api_key: String) -> anyhow::Result<()> {
    let embed_ctx = Arc::new(Mutex::new(EmbeddingContext::new()?));
    let mut graph = Graph::new(embed_ctx);

    load_server_graph(&mut graph)?;

    let pipeline = Arc::new(Mutex::new(IngestionPipeline::new(graph)));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let gmail_progress = Arc::new(GmailSyncProgress::new_idle());

    let state = AppState {
        pipeline,
        api_key,
        oauth_csrf: Arc::new(Mutex::new(None)),
        gmail_progress,
    };

    let app = Router::new()
        .route("/ingest/pdf", post(ingest_pdf))
        .route("/graph/meta", get(get_graph_meta))
        .route("/graph/nodes", get(get_graph_nodes_page))
        .route("/graph/edges_subset", post(post_graph_edges_subset))
        .route("/graph", get(get_graph))
        .route("/chat", post(chat))
        .route("/connect/gmail/callback", get(connect_gmail_callback))
        .route("/connect/gmail/status", get(connect_gmail_status))
        .route("/connect/gmail", get(connect_gmail_start))
        .route("/sync/gmail/progress", get(sync_gmail_progress))
        .route("/sync/gmail", post(sync_gmail))
        .route("/workspace/projects", get(workspace_list_projects))
        .route("/workspace/archive", post(workspace_archive))
        .route("/workspace/reset", post(workspace_reset))
        .route("/workspace/load", post(workspace_load))
        .route("/workspace/delete", post(workspace_delete))
        .layer(cors)
        .with_state(state);

    let listner = tokio::net::TcpListener::bind("0.0.0.0:8001").await?;

    println!("KG-GRAPH Listening on http://localhost:8001");
    println!("  Gmail OAuth: GET /connect/gmail (JSON) or ?redirect=1 (302); ?force_consent=1 always shows consent");
    println!("  Gmail callback: GET /connect/gmail/callback");
    println!("  Gmail sync: POST /sync/gmail (202) + GET /sync/gmail/progress");
    println!("  GET /graph — capped sample; GET /graph/meta + /graph/nodes + POST /graph/edges_subset for UI paging");
    println!("  Workspace projects: GET /workspace/projects; POST /workspace/archive|reset|load|delete (JSON body {{\"id\":\"...\"}} where needed)");

    axum::serve(listner, app).await?;

    Ok(())
}

// ---- GET /connect/gmail
#[derive(Deserialize)]
struct ConnectGmailQuery {
    /// If `"1"` or `"true"`, respond with 302 to Google (browser flow).
    redirect: Option<String>,
    /// If set, always add `prompt=consent` (new refresh token after revoke, etc.).
    force_consent: Option<String>,
}

fn query_flag_truthy(v: &Option<String>) -> bool {
    matches!(
        v.as_deref().map(str::trim),
        Some("1" | "true" | "yes" | "on")
    )
}

async fn connect_gmail_start(
    State(state): State<AppState>,
    Query(q): Query<ConnectGmailQuery>,
) -> Result<Response, (StatusCode, String)> {
    let force_consent = query_flag_truthy(&q.force_consent);
    let oauth = get_auth_url(force_consent).map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;
    *state.oauth_csrf.lock().unwrap() = Some(oauth.csrf_state.clone());

    let do_redirect = query_flag_truthy(&q.redirect);

    if do_redirect {
        Ok(Redirect::temporary(&oauth.url).into_response())
    } else {
        Ok(Json(serde_json::json!({
            "url": oauth.url,
            "state": oauth.csrf_state,
        }))
        .into_response())
    }
}

// ---- GET /connect/gmail/status
async fn connect_gmail_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "connected": credentials_exist() }))
}

// ---- GET /connect/gmail/callback
#[derive(Deserialize)]
struct GmailCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn connect_gmail_callback(
    State(state): State<AppState>,
    Query(q): Query<GmailCallbackQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    if let Some(err) = &q.error {
        let desc = q
            .error_description
            .as_deref()
            .unwrap_or("")
            .replace('<', "")
            .replace('>', "");
        return Ok(Html(format!(
            "<!DOCTYPE html><html><body style=\"font-family:system-ui;padding:2rem\">\
             <h1>OAuth error</h1><p><code>{}</code></p><p>{}</p></body></html>",
            err, desc
        )));
    }

    let code = q
        .code
        .ok_or((StatusCode::BAD_REQUEST, "missing code".to_string()))?;
    let state_param = q
        .state
        .ok_or((StatusCode::BAD_REQUEST, "missing state".to_string()))?;

    let expected = state.oauth_csrf.lock().unwrap().take();
    match expected {
        Some(e) if e == state_param => {}
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "invalid OAuth state — open /connect/gmail again".to_string(),
            ));
        }
    }

    exchange_code(&code)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Html(
        "<!DOCTYPE html><html><body style=\"font-family:system-ui;padding:2rem\">\
         <h1>Gmail connected</h1>\
         <p>Token saved to <code>~/.fluvio/credentials/gmail.json</code>.</p>\
         <p>You can close this tab and return to Fluvio, then run <strong>Sync</strong>.</p>\
         </body></html>"
            .to_string(),
    ))
}

// ---- POST /sync/gmail
#[derive(Deserialize)]
struct GmailSyncBody {
    #[serde(default = "default_gmail_sync_mode")]
    mode: String,
    /// Cap `users.threads.list` for full sync and for incremental first-run bootstrap.
    #[serde(default)]
    max_threads: Option<u32>,
    /// Cap `users.messages.list` (query mode) and max new messages per incremental history walk.
    #[serde(default)]
    max_messages: Option<u32>,
    /// Gmail `q` for thread listing in full sync (and incremental bootstrap if `bootstrap_query` unset).
    #[serde(default)]
    thread_query: Option<String>,
    /// When incremental has no stored `historyId`, list threads with this `q` instead of whole mailbox.
    #[serde(default)]
    bootstrap_query: Option<String>,
}

fn default_gmail_sync_mode() -> String {
    "incremental".to_string()
}

/// Poll while `POST /sync/gmail` is running (same fields as the final result summary when done).
async fn sync_gmail_progress(State(state): State<AppState>) -> Json<GmailSyncProgressSnapshot> {
    Json(state.gmail_progress.snapshot())
}

/// Starts Gmail sync in the background and returns **202** — poll `GET /sync/gmail/progress` for `percent` / `phase`.
async fn sync_gmail(
    State(state): State<AppState>,
    Json(body): Json<GmailSyncBody>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mode = body.mode.clone();
    let max_threads = body.max_threads;
    let max_messages = body.max_messages;
    let thread_query = body.thread_query.clone();
    let bootstrap_query = body.bootstrap_query.clone();

    if state.gmail_progress.try_begin(&mode).is_err() {
        return Err((
            StatusCode::CONFLICT,
            "Gmail sync is already running — wait for it to finish or poll /sync/gmail/progress".to_string(),
        ));
    }

    let progress = state.gmail_progress.clone();
    let pipeline = state.pipeline.clone();

    tokio::spawn(async move {
        let progress_in_block = progress.clone();
        let join = tokio::task::spawn_blocking(move || {
            let mut connector = GmailConnector::new().with_progress(progress_in_block);
            if let Some(t) = max_threads {
                connector = connector.with_max_threads(t);
            }
            if let Some(m) = max_messages {
                connector = connector.with_max_message_fetch(m);
            }
            if let Some(q) = thread_query {
                connector = connector.with_thread_query(q);
            }
            if let Some(b) = bootstrap_query {
                connector = connector.with_bootstrap_query(b);
            }
            connector.extract(&mode)
        });

        let extracted = join.await;
        match extracted {
            Ok(Ok(chunks)) => {
                progress.set_building_graph();
                let chunk_count = chunks.len();
                let ingest_result = (|| {
                    let mut pipeline = pipeline.lock().map_err(|e| e.to_string())?;
                    let (nodes_added, structured_edges) = pipeline
                        .ingest_normalized_chunks(&chunks)
                        .map_err(|e| e.to_string())?;
                    pipeline.wire_edges(0.35);
                    let graph_nodes = pipeline.graph.nodes.len();
                    let graph_edges: usize =
                        pipeline.graph.adj_list.values().map(|e| e.len()).sum();
                    persist_workspace_snapshots(&pipeline.graph).map_err(|e| e.to_string())?;
                    Ok::<GmailSyncResultSummary, String>(GmailSyncResultSummary {
                        chunks: chunk_count,
                        nodes_added,
                        structured_edges,
                        graph_nodes,
                        graph_edges,
                    })
                })();

                match ingest_result {
                    Ok(summary) => progress.finish_ok(summary),
                    Err(e) => progress.finish_err(e),
                }
            }
            Ok(Err(e)) => progress.finish_err(e.to_string()),
            Err(e) => progress.finish_err(e.to_string()),
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "poll": "/sync/gmail/progress",
            "hint": "Poll until running is false; then read result or error on the same JSON."
        })),
    ))
}

// ---- POST /ingest/pdf
#[derive(serde::Serialize)]
struct IngestResponse {
    nodes: usize,
    edges: usize,
}

async fn ingest_pdf(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<IngestResponse>, (axum::http::StatusCode, String)> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?
        .ok_or((
            axum::http::StatusCode::BAD_REQUEST,
            "no file".to_string(),
        ))?;

    let bytes = field
        .bytes()
        .await
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
        if chunk.trim().is_empty() {
            continue;
        }
        let _ = pipeline.ingest_chunk(chunk, "pdf", i + 1);
    }

    pipeline.wire_edges(0.35);
    persist_workspace_snapshots(&pipeline.graph).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
    })?;

    let total_edges: usize = pipeline.graph.adj_list.values().map(|e| e.len()).sum();

    Ok(Json(IngestResponse {
        nodes: pipeline.graph.nodes.len(),
        edges: total_edges,
    }))
}

// ---- Get /graph
#[derive(Deserialize)]
struct GraphListQuery {
    /// Max nodes for UI visualization (default 700). Use `0` for the full graph (can be very large/slow).
    #[serde(default)]
    max_nodes: Option<usize>,
    /// When set (`pdf`, `email`, …), only nodes whose `metadata["source"]` match (`email` also matches legacy `gmail`).
    #[serde(default)]
    source: Option<String>,
}

fn source_filter_matches(meta_source: &str, filter: &str) -> bool {
    let f = filter.trim().to_ascii_lowercase();
    let s = meta_source.to_ascii_lowercase();
    match f.as_str() {
        "email" => s == "email" || s == "gmail",
        _ => s == f,
    }
}

/// When the graph is larger than `cap`, round-robin across `source` buckets so one domain does not
/// crowd out others in the UI payload (e.g. after a large Gmail sync).
fn stratified_graph_nodes(mut items: Vec<GraphNode>, cap: usize) -> Vec<GraphNode> {
    if items.len() <= cap {
        return items;
    }

    let mut buckets: HashMap<String, VecDeque<GraphNode>> = HashMap::new();
    for n in items.drain(..) {
        let k = if n.source.is_empty() {
            "_other".to_string()
        } else {
            n.source.clone()
        };
        buckets.entry(k).or_default().push_back(n);
    }

    let mut keys: Vec<String> = buckets.keys().cloned().collect();
    keys.sort_unstable();

    let mut out = Vec::with_capacity(cap);
    let mut idx = 0usize;
    while out.len() < cap {
        let mut progressed = false;
        for _ in 0..keys.len() {
            let k = &keys[idx % keys.len()];
            idx += 1;
            if let Some(q) = buckets.get_mut(k) {
                if let Some(n) = q.pop_front() {
                    out.push(n);
                    progressed = true;
                    if out.len() >= cap {
                        break;
                    }
                }
            }
        }
        if !progressed {
            break;
        }
    }
    out
}

#[derive(serde::Serialize)]
struct GraphResponse {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    graph_total_nodes: usize,
    graph_returned_nodes: usize,
    graph_total_edges: usize,
    graph_returned_edges: usize,
    /// Counts over the **full** in-memory graph (not only the returned sample).
    source_counts: HashMap<String, usize>,
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

fn graph_totals_and_sources(graph: &Graph) -> (usize, usize, HashMap<String, usize>) {
    let total_nodes = graph.nodes.len();
    let total_edges: usize = graph.adj_list.values().map(|e| e.len()).sum();
    let mut source_counts = HashMap::new();
    for n in graph.nodes.values() {
        let s = n
            .metadata
            .get("source")
            .cloned()
            .filter(|x| !x.is_empty())
            .unwrap_or_else(|| "_other".into());
        *source_counts.entry(s).or_insert(0) += 1;
    }
    (total_nodes, total_edges, source_counts)
}

#[derive(Serialize)]
struct GraphMetaResponse {
    graph_total_nodes: usize,
    graph_total_edges: usize,
    source_counts: HashMap<String, usize>,
}

#[derive(Deserialize)]
struct GraphNodesPageQuery {
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Serialize)]
struct GraphNodesPageResponse {
    nodes: Vec<GraphNode>,
    offset: usize,
    limit: usize,
    total_nodes: usize,
    returned: usize,
    done: bool,
}

#[derive(Deserialize)]
struct GraphEdgesSubsetRequest {
    ids: Vec<String>,
    /// Hard cap on how many edges are returned (default keeps browser JSON.parse + layout tractable).
    #[serde(default)]
    max_edges: Option<usize>,
}

#[derive(Serialize)]
struct GraphEdgesSubsetResponse {
    edges: Vec<GraphEdge>,
    truncated: bool,
    returned_edges: usize,
}

async fn get_graph_meta(State(state): State<AppState>) -> Json<GraphMetaResponse> {
    let pipeline = state.pipeline.lock().unwrap();
    let (graph_total_nodes, graph_total_edges, source_counts) = graph_totals_and_sources(&pipeline.graph);
    Json(GraphMetaResponse {
        graph_total_nodes,
        graph_total_edges,
        source_counts,
    })
}

async fn get_graph_nodes_page(
    State(state): State<AppState>,
    Query(q): Query<GraphNodesPageQuery>,
) -> Result<Json<GraphNodesPageResponse>, (StatusCode, String)> {
    let pipeline = state.pipeline.lock().unwrap();
    let offset = q.offset.unwrap_or(0);
    let limit = q.limit.unwrap_or(400).clamp(1, 800);
    let graph = &pipeline.graph;
    let total_nodes = graph.nodes.len();
    if offset >= total_nodes {
        return Ok(Json(GraphNodesPageResponse {
            nodes: vec![],
            offset,
            limit,
            total_nodes,
            returned: 0,
            done: true,
        }));
    }
    let mut ids: Vec<_> = graph.nodes.keys().copied().collect();
    ids.sort_by_key(|u| u.to_string());
    let end = (offset + limit).min(total_nodes);
    let slice = &ids[offset..end];
    let nodes: Vec<GraphNode> = slice
        .iter()
        .filter_map(|id| graph.nodes.get(id))
        .map(|n| {
            let label: String = n.source_text.chars().take(60).collect();
            GraphNode {
                id: n.id.to_string(),
                label,
                page: n.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string()),
                source: n.metadata.get("source").cloned().unwrap_or_default(),
            }
        })
        .collect();
    let returned = nodes.len();
    let done = offset + returned >= total_nodes;
    Ok(Json(GraphNodesPageResponse {
        nodes,
        offset,
        limit,
        total_nodes,
        returned,
        done,
    }))
}

async fn post_graph_edges_subset(
    State(state): State<AppState>,
    Json(body): Json<GraphEdgesSubsetRequest>,
) -> Result<Json<GraphEdgesSubsetResponse>, (StatusCode, String)> {
    const MAX_IDS: usize = 12_000;
    const ABS_MAX_OUT: usize = 250_000;
    if body.ids.len() > MAX_IDS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("at most {MAX_IDS} node ids per request"),
        ));
    }
    let edge_cap = body
        .max_edges
        .unwrap_or(48_000)
        .clamp(1, ABS_MAX_OUT);
    let id_set: HashSet<String> = body.ids.into_iter().collect();
    let pipeline = state.pipeline.lock().unwrap();
    let graph = &pipeline.graph;
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut truncated = false;
    'outer: for e in graph.adj_list.values().flatten() {
        let a = e.from.to_string();
        let b = e.to.to_string();
        if !id_set.contains(&a) || !id_set.contains(&b) {
            continue;
        }
        edges.push(GraphEdge {
            from: a,
            to: b,
            token: e.token,
            probability: e.relationship_probability,
        });
        if edges.len() >= edge_cap {
            truncated = true;
            break 'outer;
        }
    }
    let returned_edges = edges.len();
    Ok(Json(GraphEdgesSubsetResponse {
        edges,
        truncated,
        returned_edges,
    }))
}

async fn get_graph(
    Query(q): Query<GraphListQuery>,
    State(state): State<AppState>,
) -> Json<GraphResponse> {
    let pipeline = state.pipeline.lock().unwrap();

    let cap = match q.max_nodes {
        None => 700,
        Some(0) => usize::MAX,
        Some(n) => n.max(1),
    };

    let filter = q
        .source
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let (graph_total_nodes, graph_total_edges, source_counts) =
        graph_totals_and_sources(&pipeline.graph);

    let mut nodes: Vec<GraphNode> = pipeline
        .graph
        .nodes
        .values()
        .filter(|n| {
            filter.as_ref().map_or(true, |f| {
                let src = n.metadata.get("source").map(String::as_str).unwrap_or("");
                source_filter_matches(src, f)
            })
        })
        .map(|n| {
            let label: String = n.source_text.chars().take(60).collect();

            GraphNode {
                id: n.id.to_string(),
                label,
                page: n.metadata.get("page").cloned().unwrap_or_else(|| "?".to_string()),
                source: n
                    .metadata
                    .get("source")
                    .cloned()
                    .unwrap_or_default(),
            }
        })
        .collect();

    let matched = nodes.len();

    let id_set: HashSet<String> = if cap < matched {
        if filter.is_some() {
            nodes.sort_by(|a, b| a.id.cmp(&b.id));
            nodes.truncate(cap);
        } else {
            nodes = stratified_graph_nodes(nodes, cap);
        }
        nodes.iter().map(|n| n.id.clone()).collect()
    } else {
        nodes.iter().map(|n| n.id.clone()).collect()
    };

    let edges: Vec<GraphEdge> = pipeline
        .graph
        .adj_list
        .values()
        .flatten()
        .filter(|e| {
            let a = e.from.to_string();
            let b = e.to.to_string();
            id_set.contains(&a) && id_set.contains(&b)
        })
        .map(|e| GraphEdge {
            from: e.from.to_string(),
            to: e.to.to_string(),
            token: e.token,
            probability: e.relationship_probability,
        })
        .collect();

    let returned_nodes = nodes.len();
    let returned_edges = edges.len();

    Json(GraphResponse {
        nodes,
        edges,
        graph_total_nodes,
        graph_returned_nodes: returned_nodes,
        graph_total_edges,
        graph_returned_edges: returned_edges,
        source_counts,
    })
}

// POST --- /chat
#[derive(serde::Deserialize)]
struct ChatRequest {
    question: String,
    history: Vec<HistoryMessage>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct HistoryMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize)]
struct ChatResponse {
    answer: String,
    sources: Vec<SourceNode>,
}

#[derive(serde::Serialize)]
struct SourceNode {
    id: String,
    page: String,
    score: f32,
    text: String,
}

#[axum::debug_handler]
async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    // ── do all graph work under the lock, then drop it ────────────────────────
    let (_query_vec, context, sources) = {
        let pipeline = state.pipeline.lock().unwrap();

        let query_vec = pipeline
            .graph
            .embed_ctx
            .lock()
            .unwrap()
            .embed(&req.question)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let kg = KnowledgeGraphQuery::new(&pipeline.graph);
        let (results, context) = kg.search_with_relational_context(&query_vec, 6, 5);

        if results.is_empty() {
            return Ok(Json(ChatResponse {
                answer: "I could not find relevant content in the graph.".to_string(),
                sources: vec![],
            }));
        }

        let sources: Vec<SourceNode> = results
            .iter()
            .map(|r| SourceNode {
                id: r.id.to_string(),
                page: r
                    .metadata
                    .get("page")
                    .cloned()
                    .unwrap_or_else(|| "?".to_string()),
                score: r.score,
                text: r.source_text.chars().take(120).collect(),
            })
            .collect();

        (query_vec, context, sources)
    }; // ← lock drops here, before any await

    // ── everything below is lock-free ─────────────────────────────────────────
    let mut messages: Vec<serde_json::Value> = req
        .history
        .iter()
        .map(|h| serde_json::json!({"role": h.role, "content": h.content}))
        .collect();
    messages.push(serde_json::json!({"role": "user", "content": req.question}));

    let system = format!(
        "You are a helpful assistant answering questions using the user's knowledge graph.\n\
         The context lists semantically retrieved seed nodes and their outgoing edges, including relationship_probability and token_cost on each edge.\n\
         Answer using ONLY this graph context. Be concise.\n\
         If the answer is not supported by the context, say \"I don't see that in the knowledge graph context.\"\n\n\
         KNOWLEDGE GRAPH CONTEXT:\n{context}"
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
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let answer = res["content"][0]["text"]
        .as_str()
        .unwrap_or("(no response)")
        .to_string();

    Ok(Json(ChatResponse { answer, sources }))
}

// ---- Workspace projects (saved under fluvio_graphs/projects/<id>/)

#[derive(Deserialize)]
struct ProjectIdBody {
    id: String,
}

async fn workspace_list_projects() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    std::fs::create_dir_all(WORKSPACE_PROJECTS_DIR)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut projects: Vec<serde_json::Value> = Vec::new();
    let rd = std::fs::read_dir(WORKSPACE_PROJECTS_DIR)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    for ent in rd {
        let ent = ent.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let ty = ent
            .file_type()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if ty.is_dir() {
            let name = ent.file_name().to_string_lossy().to_string();
            if !name.is_empty() && name != "." && name != ".." {
                projects.push(serde_json::json!({"id": name}));
            }
        }
    }
    projects.sort_by(|a, b| {
        a["id"]
            .as_str()
            .unwrap_or("")
            .cmp(b["id"].as_str().unwrap_or(""))
    });
    Ok(Json(serde_json::json!({ "projects": projects })))
}

async fn workspace_archive(
    State(state): State<AppState>,
    Json(body): Json<ProjectIdBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = sanitize_project_id(&body.id)?;
    let dest = format!("{WORKSPACE_PROJECTS_DIR}/{id}");
    if Path::new(&dest).exists() {
        return Err((
            StatusCode::CONFLICT,
            format!("project '{id}' already exists — pick another id or delete it first"),
        ));
    }
    {
        let p = state
            .pipeline
            .lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        persist_workspace_snapshots(&p.graph)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    std::fs::create_dir_all(&dest).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    for (src, fname) in [
        (WORKSPACE_UNIFIED, "unified.json"),
        (WORKSPACE_PDF, "pdf.json"),
        (WORKSPACE_EMAIL, "email.json"),
    ] {
        if Path::new(src).exists() {
            let to = format!("{dest}/{fname}");
            std::fs::copy(src, &to).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
    }
    Ok(Json(serde_json::json!({ "ok": true, "id": id, "path": dest })))
}

async fn workspace_reset(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut p = state
        .pipeline
        .lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    p.graph.clear();
    persist_workspace_snapshots(&p.graph).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true, "nodes": 0, "edges": 0 })))
}

async fn workspace_load(
    State(state): State<AppState>,
    Json(body): Json<ProjectIdBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = sanitize_project_id(&body.id)?;
    let src = format!("{WORKSPACE_PROJECTS_DIR}/{id}/unified.json");
    if !Path::new(&src).is_file() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("no unified.json for project '{id}'"),
        ));
    }
    let mut p = state
        .pipeline
        .lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    p.graph
        .load(&src)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    persist_workspace_snapshots(&p.graph).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let nodes = p.graph.nodes.len();
    let edges: usize = p.graph.adj_list.values().map(|e| e.len()).sum();
    Ok(Json(serde_json::json!({ "ok": true, "id": id, "nodes": nodes, "edges": edges })))
}

async fn workspace_delete(
    State(_state): State<AppState>,
    Json(body): Json<ProjectIdBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let id = sanitize_project_id(&body.id)?;
    let dest = format!("{WORKSPACE_PROJECTS_DIR}/{id}");
    if !Path::new(&dest).exists() {
        return Err((StatusCode::NOT_FOUND, format!("project '{id}' not found")));
    }
    std::fs::remove_dir_all(&dest).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true, "id": id })))
}
