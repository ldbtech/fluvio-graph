// ---- GET /codebase/parse
use std::collections::{HashMap, HashSet};
use std::path::Path;

use axum::{extract::Query, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ingestion_registry::{
    codebase::{resolver::ResolvedGraph, CodebaseConnector, RepoRef},
    ConnectorError,
};
use crate::app_state::AppState;
use crate::authentication::AuthUser;
use crate::database::user_uploads::{
    delete_user_uploads_by_kind, insert_user_upload, list_user_uploads_for_user_by_kind,
};
use crate::graph::structs::NodeId;
use axum::extract::State;

/// `codebase://github.com/{owner}/{repo}/` + repo-relative tail (`path` or `path#symbol`).
fn codebase_uri_repo_tail(uri: &str, owner: &str, repo: &str) -> Option<String> {
    let prefix = format!("codebase://github.com/{owner}/{repo}/");
    let rest = uri.strip_prefix(&prefix)?;
    Some(rest.replace('\\', "/"))
}

/// UI + LLM subgraph: every chunk in the resolve batch (file + symbols) plus structured edges.
/// `pipeline_nodes` is the whole workspace graph size after merge; this payload is only the
/// import + containment slice for the resolved files.
fn build_import_subgraph_from_resolved(
    resolved: &ResolvedGraph,
    repo_url: &str,
) -> Result<(Vec<ResolveGraphNode>, Vec<ResolveGraphEdge>), (StatusCode, String)> {
    let repo_ref = RepoRef::parse(repo_url.trim())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let owner = repo_ref.owner.as_str();
    let repo_name = repo_ref.repo.as_str();

    let mut node_map: HashMap<String, ResolveGraphNode> = HashMap::new();

    for chunk in &resolved.chunks {
        let Some(tail) = codebase_uri_repo_tail(&chunk.source_uri, owner, repo_name) else {
            continue;
        };
        let path = chunk
            .metadata
            .get("path")
            .map(|s| s.replace('\\', "/"))
            .unwrap_or_else(|| {
                tail.split('#')
                    .next()
                    .unwrap_or_default()
                    .to_string()
            });
        let kind = chunk.metadata.get("kind").map(|s| s.as_str()).unwrap_or("");
        let is_file_chunk = kind == "file" || (kind.is_empty() && !tail.contains('#'));
        let label = if is_file_chunk {
            Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone())
        } else {
            chunk
                .metadata
                .get("name")
                .cloned()
                .or_else(|| {
                    tail.rsplit_once('#')
                        .map(|(_, n)| n.to_string())
                })
                .unwrap_or_else(|| tail.clone())
        };

        node_map.entry(tail.clone()).or_insert(ResolveGraphNode {
            id:     tail,
            label,
            page:   path,
            source: "github".to_string(),
        });
    }

    // `defined_in` (symbol → file) duplicates `contains` (file → symbol) for layout; keep the tree edge only.
    let mut edges_out: Vec<ResolveGraphEdge> = Vec::new();
    let mut seen_edge: HashSet<(String, String, String)> = HashSet::new();

    for chunk in &resolved.chunks {
        let Some(from_tail) = codebase_uri_repo_tail(&chunk.source_uri, owner, repo_name) else {
            continue;
        };
        if !node_map.contains_key(&from_tail) {
            continue;
        }

        for e in &chunk.pre_defined_edges {
            if e.label != "imports" && e.label != "contains" {
                continue;
            }
            let Some(to_tail) = codebase_uri_repo_tail(&e.to_uri, owner, repo_name) else {
                continue;
            };
            if !node_map.contains_key(&to_tail) {
                continue;
            }
            let token = if e.token_cost > 0 { e.token_cost } else { 1 };
            if seen_edge.insert((from_tail.clone(), to_tail.clone(), e.label.clone())) {
                edges_out.push(ResolveGraphEdge {
                    from:          from_tail.clone(),
                    to:            to_tail,
                    token,
                    probability:   e.relationship_probability,
                    label:         e.label.clone(),
                });
            }
        }
    }

    let mut graph_nodes: Vec<ResolveGraphNode> = node_map.into_values().collect();
    graph_nodes.sort_by(|a, b| {
        let a_sym = a.id.contains('#');
        let b_sym = b.id.contains('#');
        a_sym
            .cmp(&b_sym)
            .then_with(|| a.id.cmp(&b.id))
    });

    Ok((graph_nodes, edges_out))
}

#[derive(Deserialize)]
pub struct CodebaseParseQuery {
    url: String,
    path: String,
}

#[derive(Serialize)]
pub struct CodebaseParseResponse {
    path: String,
    language: String,
    imports: Vec<serde_json::Value>,
    symbols: Vec<serde_json::Value>,
}

pub async fn get_codebase_parse(
    Query(q): Query<CodebaseParseQuery>,
) -> Result<Json<CodebaseParseResponse>, (StatusCode, String)> {
    let url = q.url.trim().to_string();
    let path = q.path.trim().to_string();
    let path_for_extract = path.clone();

    let join = tokio::task::spawn_blocking(move || {
        CodebaseConnector::extract_file(&url, &path_for_extract)
    });

    let chunks = join
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    .map_err(|e| {
                        let status = match &e {
                            ConnectorError::Parse(_)                      => StatusCode::BAD_REQUEST,
                            ConnectorError::NotConfigured(_)              => StatusCode::NOT_FOUND,
                            _                                             => StatusCode::INTERNAL_SERVER_ERROR,
                        };
                        (status, e.to_string())
                    })?;
    
    // pull the first chunk (file-level) for the response summary.
    let file_chunk = chunks.first()
                    .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "no chunks produced".to_string()))?;
    
    let language = file_chunk.metadata
                    .get("language").cloned().unwrap_or_default();
    

    // Collect import edges from the file chunk
    let imports: Vec<serde_json::Value> = file_chunk
                        .pre_defined_edges.iter()
                        .filter(|e| e.label == "imports")
                        .map(|e| serde_json::json!({
                            "to_uri":       e.to_uri,
                            "label":        e.label,
                            "probability":   e.relationship_probability,
                        }))
                        .collect();
    
    // collect symbol chunks
    let symbols: Vec<serde_json::Value> = chunks.iter().skip(1)
                        .filter(|c| c.metadata.get("kind")
                                           .map(|k| k != "file").unwrap_or(false))
                        .map(|c| serde_json::json! ({
                            "name":                      c.metadata.get("name").cloned().unwrap_or_default(),
                            "kind":                      c.metadata.get("kind").cloned().unwrap_or_default(),
                            "signature":                 c.metadata.get("signature").cloned().unwrap_or_default(),
                            "line":                      c.metadata.get("line").cloned().unwrap_or_default(),
                            "is_public":                 c.metadata.get("is_public").cloned().unwrap_or_default(),

                        }))
                        .collect();

    Ok(Json(CodebaseParseResponse {
        path,
        language,
        imports,
        symbols,
    }))
}

#[derive(Deserialize)]
pub struct CodebaseTreeQuery {
    url: Option<String>,
    owner: Option<String>,
    repo: Option<String>,
}

pub async fn get_codebase_tree(
    Query(q): Query<CodebaseTreeQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let url = if let Some(u) = q.url.filter(|s| !s.trim().is_empty()) {
        u
    } else {
        let o = q
            .owner
            .filter(|s| !s.trim().is_empty())
            .ok_or((StatusCode::BAD_REQUEST, "pass url= or owner=&repo=".to_string()))?;
        let r = q
            .repo
            .filter(|s| !s.trim().is_empty())
            .ok_or((StatusCode::BAD_REQUEST, "pass url= or owner=&repo=".to_string()))?;
        format!("{o}/{r}")
    };

    let join = tokio::task::spawn_blocking(move || crate::ingestion_registry::codebase::tree::build_tree(&url));

    let tree = join
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| {
            let status = match &e {
                crate::ingestion_registry::codebase::tree::TreeError::NotCloned(_) => StatusCode::NOT_FOUND,
                crate::ingestion_registry::codebase::tree::TreeError::InvalidUrl(_) => StatusCode::BAD_REQUEST,
                crate::ingestion_registry::codebase::tree::TreeError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, e.to_string())
        })?;

    Ok(Json(
        serde_json::to_value(tree).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
    ))
}

// POST /codebase/clone and POST /sync/codebase/clone — shallow clone or pull; required before POST /ingest on a new machine.

#[derive(Deserialize)]
pub struct CodebaseCloneBody {
    pub url: String,
}

#[derive(Serialize)]
pub struct CodebaseCloneResponse {
    pub owner: String,
    pub repo: String,
    pub local_path: String,
    pub was_cloned: bool,
}

pub async fn post_codebase_clone(
    Json(body): Json<CodebaseCloneBody>,
) -> Result<Json<CodebaseCloneResponse>, (StatusCode, String)> {
    let url = body.url.trim().to_string();
    if url.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "url required".to_string()));
    }

    let join = tokio::task::spawn_blocking(move || CodebaseConnector::clone_public_url(&url));

    let result = join
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e: ConnectorError| {
            let status = match &e {
                ConnectorError::Parse(_) => StatusCode::BAD_REQUEST,
                ConnectorError::Api(_) => StatusCode::BAD_GATEWAY,
                ConnectorError::NotConfigured(_) => StatusCode::SERVICE_UNAVAILABLE,
                ConnectorError::Auth(_) => StatusCode::UNAUTHORIZED,
                ConnectorError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, e.to_string())
        })?;

    Ok(Json(CodebaseCloneResponse {
        owner: result.owner,
        repo: result.repo,
        local_path: result.local_path.to_string_lossy().into_owned(),
        was_cloned: result.was_cloned,
    }))
}

#[derive(Deserialize)]
pub struct CodebaseIngestQuery {
    url: String,
    /// Relative path to ingest - can be a file or a directory prefix. 
    /// "src/graph/structs.rs" -> single file.
    /// "src/graph" -> all files under src/graph/
    /// ""
    path: String,
}

#[derive(Serialize)]
pub struct CodebaseIngestResponse {
    chunks: usize,
    nodes: usize,
    edges: usize,
}

/// Drop in-memory + Surreal nodes tagged with `metadata.codebase_scope == scope` for this user.
async fn purge_codebase_scope_for_user(
    state:   &AppState,
    user_id: Uuid,
    scope:   &str,
) -> Result<(), (StatusCode, String)> {
    let mut surreal_ids: Vec<NodeId> = state
        .surreal_storage
        .get_user_nodes(user_id, Some("Codebase"), 1)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .into_iter()
        .filter(|r| {
            r.metadata
                .get("codebase_scope")
                .map(|v| v == scope)
                .unwrap_or(false)
        })
        .map(|r| r.to_node().id)
        .collect();

    let pipeline_ids = {
        let mut pipeline = state
            .pipeline
            .lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let ids: Vec<NodeId> = pipeline
            .graph
            .nodes
            .values()
            .filter(|n| {
                n.metadata
                    .get("codebase_scope")
                    .map(|v| v == scope)
                    .unwrap_or(false)
            })
            .map(|n| n.id)
            .collect();
        let _ = pipeline.graph.remove_nodes_by_metadata("codebase_scope", scope);
        ids
    };

    surreal_ids.extend(pipeline_ids);
    surreal_ids.sort_by_key(|id| id.0);
    surreal_ids.dedup_by_key(|id| id.0);

    if !surreal_ids.is_empty() {
        state
            .surreal_storage
            .delete_node_records(&surreal_ids)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    Ok(())
}

pub async fn post_codebase_ingest(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<CodebaseIngestQuery>,
) -> Result<Json<CodebaseIngestResponse>, (StatusCode, String)> {
    let url = body.url.trim().to_string();
    let path = body.path.trim().to_string();

    let repo = RepoRef::parse(&url).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let scope = repo.key();

    let existing = list_user_uploads_for_user_by_kind(&state.pg_pool, user.id, "codebase", 50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut scope_set: HashSet<String> = existing
        .iter()
        .filter_map(|r| r.document_id.clone())
        .collect();
    scope_set.insert(scope.clone());

    for s in scope_set {
        purge_codebase_scope_for_user(&state, user.id, &s).await?;
    }

    delete_user_uploads_by_kind(&state.pg_pool, user.id, "codebase")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let url_for_library = url.clone();
    let join = tokio::task::spawn_blocking(move || {
        let connector = CodebaseConnector::new();
        connector.extract_under_prefix(&url, &path)
    });

    let chunks = join
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .map_err(|e| {
                let status = match &e {
                    ConnectorError::NotConfigured(_)              => StatusCode::NOT_FOUND,
                    ConnectorError::Parse(_)                      => StatusCode::BAD_REQUEST,
                    _                                             => StatusCode::INTERNAL_SERVER_ERROR,
                };
                (status, e.to_string())
            })?;

    let chunk_count = chunks.len();
    let owner_str = user.id.to_string();

    // Pipeline mutates RAM only as a working set; the lock is dropped before any `.await`.
    let (surreal_subgraph, total_nodes, total_edges) = {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let before_ids: std::collections::HashSet<NodeId> =
            pipeline.graph.nodes.keys().copied().collect();

        pipeline
            .ingest_normalized_chunks(&chunks)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let new_node_ids: Vec<NodeId> = pipeline
            .graph
            .nodes
            .keys()
            .filter(|id| !before_ids.contains(id))
            .copied()
            .collect();

        if !new_node_ids.is_empty() {
            pipeline.wire_edges_for_nodes(&new_node_ids, 0.35);
        }

        for id in &new_node_ids {
            if let Some(n) = pipeline.graph.nodes.get_mut(id) {
                n.metadata.insert("owner_id".into(), owner_str.clone());
                n.metadata.insert("zone".into(), "1".into());
                n.metadata.insert("codebase_scope".into(), scope.clone());
                n.metadata
                    .entry("kind".into())
                    .or_insert_with(|| "codebase".into());
            }
        }

        let surreal_subgraph = pipeline
            .graph
            .subgraph_closed(new_node_ids.iter().copied());
        let total_nodes = pipeline.graph.nodes.len();
        let total_edges: usize = pipeline.graph.adj.values().map(|e| e.len()).sum();

        (surreal_subgraph, total_nodes, total_edges)
    };

    if !surreal_subgraph.nodes.is_empty() {
        state
            .surreal_storage
            .save_graph(user.id, &surreal_subgraph, 1)
            .await
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("SurrealDB codebase persist failed: {e}"),
            ))?;

        let subgraph_nodes = surreal_subgraph.nodes.len() as i32;
        let subgraph_edges: i32 = surreal_subgraph
            .adj
            .values()
            .map(|e| e.len() as i32)
            .sum();

        if let Err(e) = insert_user_upload(
            &state.pg_pool,
            user.id,
            "codebase",
            &url_for_library,
            Some(scope.as_str()),
            subgraph_nodes,
            subgraph_edges,
        )
        .await
        {
            tracing::warn!("[DB] user_uploads codebase insert skipped: {e}");
        }
    }

    Ok(Json(CodebaseIngestResponse {
        chunks: chunk_count,
        nodes: total_nodes,
        edges: total_edges,
    }))
}

// POST /codebase/resolve
#[derive(Deserialize)]
pub struct CodebaseResolveBody {
    pub url:       String,
    pub path:      String,
    #[serde(default = "default_depth")]
    pub max_depth: usize,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_depth() -> usize {
    2
}

fn default_max_files() -> usize {
    30
}

#[derive(Serialize)]
pub struct ResolveGraphNode {
    pub id:     String,
    pub label:  String,
    pub page:   String,
    pub source: String,
}

#[derive(Serialize)]
pub struct ResolveGraphEdge {
    pub from:        String,
    pub to:          String,
    pub token:       i32,
    pub probability: f64,
    pub label:       String,
}

#[derive(Serialize)]
pub struct CodebaseResolveResponse {
    pub chunks:               usize,
    pub resolved_paths:       Vec<String>,
    pub unresolved_imports:   Vec<String>,
    pub max_depth_reached:    usize,
    pub nodes:                usize,
    pub edges:                usize,
    pub graph_nodes:          Vec<ResolveGraphNode>,
    pub graph_edges:          Vec<ResolveGraphEdge>,
}

pub async fn post_codebase_resolve(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<CodebaseResolveBody>,
) -> Result<Json<CodebaseResolveResponse>, (StatusCode, String)> {
    let url       = body.url.trim().to_string();
    let path      = body.path.trim().to_string();
    let max_depth = body.max_depth;
    let max_files = body.max_files;
    let url_for_graph = url.clone();
    let path_log = path.clone();

    eprintln!(
        "[codebase/resolve] start url={url} path={path} max_depth={max_depth} max_files={max_files}"
    );

    let join = tokio::task::spawn_blocking(move || {
        crate::ingestion_registry::codebase::resolver::resolve_file(
            &url, &path, max_depth, max_files,
        )
    });

    let resolved = join
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| {
            let status = match &e {
                ConnectorError::NotConfigured(_) => StatusCode::NOT_FOUND,
                ConnectorError::Parse(_)         => StatusCode::BAD_REQUEST,
                _                                => StatusCode::INTERNAL_SERVER_ERROR,
            };
            (status, e.to_string())
        })?;

    let chunk_count    = resolved.chunks.len();
    let resolved_paths = resolved.resolved_paths.clone();
    let unresolved     = resolved.unresolved_imports.clone();
    let depth_reached  = resolved.max_depth_reached;

    let (graph_nodes, graph_edges) =
        build_import_subgraph_from_resolved(&resolved, &url_for_graph)?;

    let scope_for_nodes = RepoRef::parse(&url_for_graph)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
        .key();

    let owner_str = user.id.to_string();

    let (surreal_subgraph, nodes, edges) = {
        let mut pipeline = state
            .pipeline
            .lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let before_ids: std::collections::HashSet<NodeId> =
            pipeline.graph.nodes.keys().copied().collect();

        pipeline
            .ingest_normalized_chunks(&resolved.chunks)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let new_node_ids: Vec<NodeId> = pipeline
            .graph
            .nodes
            .keys()
            .filter(|id| !before_ids.contains(id))
            .copied()
            .collect();

        if !new_node_ids.is_empty() {
            pipeline.wire_edges_for_nodes(&new_node_ids, 0.35);
        }

        for id in &new_node_ids {
            if let Some(n) = pipeline.graph.nodes.get_mut(id) {
                n.metadata.insert("owner_id".into(), owner_str.clone());
                n.metadata.insert("zone".into(), "1".into());
                n.metadata.insert("codebase_scope".into(), scope_for_nodes.clone());
                n.metadata
                    .entry("kind".into())
                    .or_insert_with(|| "codebase".into());
            }
        }

        let surreal_subgraph = pipeline
            .graph
            .subgraph_closed(new_node_ids.iter().copied());
        let nodes = pipeline.graph.nodes.len();
        let edges: usize = pipeline.graph.adj.values().map(|e| e.len()).sum();

        (surreal_subgraph, nodes, edges)
    };

    if !surreal_subgraph.nodes.is_empty() {
        state
            .surreal_storage
            .save_graph(user.id, &surreal_subgraph, 1)
            .await
            .map_err(|e| (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("SurrealDB codebase persist failed: {e}"),
            ))?;
    }

    eprintln!(
        "[codebase/resolve] ok url={} path={} chunks={chunk_count} resolved_files={} graph_nodes={} graph_edges={} pipeline_nodes={nodes}",
        url_for_graph,
        path_log,
        resolved_paths.len(),
        graph_nodes.len(),
        graph_edges.len(),
    );

    Ok(Json(CodebaseResolveResponse {
        chunks:               chunk_count,
        resolved_paths,
        unresolved_imports: unresolved,
        max_depth_reached:    depth_reached,
        nodes,
        edges,
        graph_nodes,
        graph_edges,
    }))
}