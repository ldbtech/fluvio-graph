// ---- GET /codebase/parse 
use axum::{http::StatusCode, Json, extract::Query};
use serde::{Deserialize, Serialize};

use crate::ingestion_registry::{ConnectorError, codebase::CodebaseConnector};
use crate::server::AppState;
use axum::extract::State;

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

pub async fn post_codebase_ingest(
    State(state): State<AppState>,
    Json(body): Json<CodebaseIngestQuery>,
) -> Result<Json<CodebaseIngestResponse>, (StatusCode, String)>{
    let url = body.url.trim().to_string();
    let path = body.path.trim().to_string();

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
    
    let (nodes, edges) = {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                
        let (nodes_added, _structured_edges) = pipeline
            .ingest_normalized_chunks(&chunks)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                
        pipeline.wire_edges(0.35);
                
        let total_nodes = pipeline.graph.nodes.len();
        let total_edges: usize = pipeline.graph.adj_list.values().map(|e| e.len()).sum();
                
        (state.presist)(&pipeline.graph)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                
        (total_nodes, total_edges)
    };
    
    Ok(Json(CodebaseIngestResponse {
        chunks: chunk_count,
        nodes,
        edges,
    }))
}