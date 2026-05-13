//! Document upload HTTP surface on kg-engine:
//! - `POST /ingest/pdf`, `POST /ingest/pdf/stream`
//! - `GET/DELETE /user/uploads` — Postgres library for PDF, video, and codebase ingests (dashboard).

use axum::{
    Json, Router,
    body::Body,
    extract::{FromRequest, Multipart, Path as AxumPath, Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::authentication::{upload_user_id_from_headers, AuthUser};
use crate::database::user_uploads::{
    delete_user_upload_row, get_user_upload_row, insert_user_upload, list_user_uploads_for_user,
    UserUpload,
};
use crate::database::users::{get_user_by_id, user_physical_scope, User};
use crate::graph::structs::NodeId;

use super::PDFChunkIterator;

pub fn pdf_ingest_router() -> Router<AppState> {
    Router::new()
        .route("/ingest/pdf", post(ingest_pdf))
        .route("/ingest/pdf/stream", post(ingest_pdf_stream))
}

pub fn user_uploads_router() -> Router<AppState> {
    Router::new()
        .route("/user/uploads", get(list_user_uploads))
        .route("/user/uploads/{id}", delete(delete_user_upload_handler))
}

#[derive(serde::Serialize)]
struct IngestResponse {
    nodes: usize,
    edges: usize,
}

/// BGESmall and similar models cap input length; huge PDF pages would fail embed or waste RAM.
const MAX_PDF_CHUNK_CHARS: usize = 24_000;

fn clamp_pdf_chunk_text(s: &str) -> String {
    if s.len() <= MAX_PDF_CHUNK_CHARS {
        return s.to_string();
    }
    s.chars().take(MAX_PDF_CHUNK_CHARS).collect()
}

struct TempUploadPdf(std::path::PathBuf);
impl Drop for TempUploadPdf {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn resolve_pdf_upload_user(
    pool: &sqlx::PgPool,
    headers: &axum::http::HeaderMap,
) -> Result<User, Response> {
    let Some(uid) = upload_user_id_from_headers(headers) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "missing upload authentication".to_string(),
        )
            .into_response());
    };
    match get_user_by_id(pool, uid).await {
        Ok(Some(u)) => Ok(u),
        Ok(None) => Err((
            StatusCode::UNAUTHORIZED,
            "session user not found".to_string(),
        )
            .into_response()),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
        )
            .into_response()),
    }
}

async fn read_pdf_upload_from_multipart(
    mut multipart: Multipart,
) -> Result<(std::path::PathBuf, String, String), (StatusCode, String)> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "no file".to_string()))?;

    let filename = field
        .file_name()
        .map(std::path::Path::new)
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "upload.pdf".to_string());
    let document_id = Uuid::new_v4().to_string();

    let bytes = field
        .bytes()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let tmp_path = std::path::PathBuf::from(format!("/tmp/fluvio_upload_{document_id}.pdf"));
    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((tmp_path, filename, document_id))
}

async fn send_pdf_ndjson_line(
    tx: &Option<mpsc::Sender<Result<Bytes, std::io::Error>>>,
    v: serde_json::Value,
) {
    let Some(t) = tx else {
        return;
    };
    let Ok(s) = serde_json::to_string(&v) else {
        return;
    };
    let _ = t.send(Ok(Bytes::from(format!("{s}\n")))).await;
}

async fn process_pdf_upload(
    state: &AppState,
    user: &User,
    tmp_path: std::path::PathBuf,
    filename: &str,
    document_id: &str,
    progress: Option<mpsc::Sender<Result<Bytes, std::io::Error>>>,
) -> Result<IngestResponse, (StatusCode, String)> {
    let tmp_extract = tmp_path.clone();
    let chunks = tokio::task::spawn_blocking(move || {
        let tmp_str = tmp_extract.to_str().ok_or_else(|| {
            "temp path is not valid UTF-8".to_string()
        })?;
        let pdf_iter = PDFChunkIterator::new(tmp_str, 1).map_err(|e| e.to_string())?;
        let mut v = Vec::new();
        for chunk_res in pdf_iter {
            v.push(chunk_res.map_err(|e| e.to_string())?);
        }
        Ok::<Vec<String>, String>(v)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("pdf extract join: {e}")))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let total_pages = chunks.len();
    let denom = total_pages.max(1);

    send_pdf_ndjson_line(
        &progress,
        serde_json::json!({
            "event": "start",
            "pages": total_pages,
            "filename": filename,
            "document_id": document_id,
        }),
    )
    .await;

    let mut new_node_ids = Vec::new();
    let mut page_seq: usize = 0;

    for (i, raw) in chunks.into_iter().enumerate() {
        page_seq += 1;
        if raw.trim().is_empty() {
            let pct = (((i + 1) * 100) / denom).min(100) as u32;
            send_pdf_ndjson_line(
                &progress,
                serde_json::json!({
                    "event": "progress",
                    "chunk_index": i + 1,
                    "pages": total_pages,
                    "percent": pct,
                    "phase": "extract",
                }),
            )
            .await;
            continue;
        }
        let chunk = clamp_pdf_chunk_text(&raw);
        let id = {
            let mut pipeline = state.pipeline.lock().unwrap();
            pipeline
                .ingest_chunk(
                    &chunk,
                    "pdf",
                    page_seq,
                    Some((document_id, filename)),
                )
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        };
        new_node_ids.push(id);

        let pct = (((i + 1) * 100) / denom).min(100) as u32;
        send_pdf_ndjson_line(
            &progress,
            serde_json::json!({
                "event": "progress",
                "chunk_index": i + 1,
                "pages": total_pages,
                "percent": pct,
                "phase": "graph",
            }),
        )
        .await;
    }

    let (surreal_subgraph, total_nodes_all, total_edges) = {
        let mut pipeline = state.pipeline.lock().unwrap();
        if !new_node_ids.is_empty() {
            pipeline.wire_edges_for_nodes(&new_node_ids, 0.35);
        }

        let scope = user_physical_scope(user);
        let owner = user.id.to_string();
        for id in &new_node_ids {
            if let Some(n) = pipeline.graph.nodes.get_mut(id) {
                n.metadata.insert("owner_id".into(), owner.clone());
                n.metadata.insert("owner_physical_id".into(), scope.clone());
                n.metadata.insert("zone".into(), "1".into());
                n.metadata
                    .entry("kind".into())
                    .or_insert_with(|| "pdf".into());
                n.metadata
                    .entry("title".into())
                    .or_insert_with(|| filename.to_string());
            }
        }
        if let Some(anchor) = new_node_ids.first() {
            if let Some(n) = pipeline.graph.nodes.get_mut(anchor) {
                n.metadata.insert("dashboard_doc_anchor".into(), "1".into());
            }
        }

        let surreal_subgraph = pipeline
            .graph
            .subgraph_closed(new_node_ids.iter().copied());
        let total_nodes_all = pipeline.graph.nodes.len();
        let total_edges: usize = pipeline.graph.adj.values().map(|e| e.len()).sum();
        (surreal_subgraph, total_nodes_all, total_edges)
    };

    send_pdf_ndjson_line(
        &progress,
        serde_json::json!({
            "event": "progress",
            "percent": 100,
            "phase": "surreal",
        }),
    )
    .await;

    if !new_node_ids.is_empty() {
        state
            .surreal_storage
            .save_graph(user.id, &surreal_subgraph, 1)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("SurrealDB PDF persist failed: {e}"),
                )
            })?;
    }

    let subgraph_nodes = surreal_subgraph.nodes.len() as i32;
    let subgraph_edges: i32 = surreal_subgraph
        .adj
        .values()
        .map(|e| e.len() as i32)
        .sum();

    if let Err(e) = insert_user_upload(
        &state.pg_pool,
        user.id,
        "pdf",
        filename,
        Some(document_id),
        subgraph_nodes,
        subgraph_edges,
    )
    .await
    {
        tracing::warn!("[DB] user_uploads insert skipped: {e}");
    }

    Ok(IngestResponse {
        nodes: total_nodes_all,
        edges: total_edges,
    })
}

async fn ingest_pdf(State(state): State<AppState>, req: Request) -> Response {
    let user = match resolve_pdf_upload_user(&state.pg_pool, req.headers()).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let multipart = match Multipart::from_request(req, &state).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    let (tmp_path, filename, document_id) = match read_pdf_upload_from_multipart(multipart).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let _tmp_cleanup = TempUploadPdf(tmp_path.clone());

    match process_pdf_upload(
        &state,
        &user,
        tmp_path,
        &filename,
        &document_id,
        None,
    )
    .await
    {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn ingest_pdf_stream(State(state): State<AppState>, req: Request) -> Response {
    let user = match resolve_pdf_upload_user(&state.pg_pool, req.headers()).await {
        Ok(u) => u,
        Err(r) => return r,
    };

    let multipart = match Multipart::from_request(req, &state).await {
        Ok(m) => m,
        Err(e) => return e.into_response(),
    };

    let (tmp_path, filename, document_id) = match read_pdf_upload_from_multipart(multipart).await {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(64);
    let state_cl = state.clone();
    let user_cl = user.clone();
    let tmp = tmp_path;
    let fname = filename.clone();
    let doc_id = document_id.clone();

    tokio::spawn(async move {
        let _cleanup = TempUploadPdf(tmp.clone());
        let tx_done = tx.clone();
        let out = process_pdf_upload(
            &state_cl,
            &user_cl,
            tmp,
            &fname,
            &doc_id,
            Some(tx.clone()),
        )
        .await;
        match out {
            Ok(resp) => {
                let line = serde_json::json!({
                    "event": "done",
                    "nodes": resp.nodes,
                    "edges": resp.edges,
                    "document_id": doc_id,
                    "filename": fname,
                });
                let s = format!(
                    "{}\n",
                    serde_json::to_string(&line).unwrap_or_else(|_| "{}".to_string())
                );
                let _ = tx_done.send(Ok(Bytes::from(s))).await;
            }
            Err((st, msg)) => {
                let line = serde_json::json!({
                    "event": "error",
                    "status": st.as_u16(),
                    "message": msg,
                });
                let s = format!(
                    "{}\n",
                    serde_json::to_string(&line).unwrap_or_else(|_| "{}".to_string())
                );
                let _ = tx_done.send(Ok(Bytes::from(s))).await;
            }
        }
    });

    match Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/x-ndjson; charset=utf-8",
        )
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(ReceiverStream::new(rx)))
    {
        Ok(resp) => resp.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("response build: {e}"),
        )
            .into_response(),
    }
}

async fn list_user_uploads(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<UserUpload>>, (StatusCode, String)> {
    let rows = list_user_uploads_for_user(&state.pg_pool, user.id, 80)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

/// Removes a library upload row, matching in-memory graph nodes (by `document_id`, `video_id`, or `codebase_scope`),
/// SurrealDB node records, and on-disk video assets when applicable.
async fn delete_user_upload_handler(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    AxumPath(upload_id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = get_user_upload_row(&state.pg_pool, user.id, upload_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "upload not found".to_string()))?;

    let mut removed_graph = 0usize;
    let mut surreal_targets: Vec<NodeId> = Vec::new();

    if let Some(ref meta_val) = row.document_id {
        let meta_key = match row.kind.as_str() {
            "pdf" => Some("document_id"),
            "video" => Some("video_id"),
            "codebase" => Some("codebase_scope"),
            _ => None,
        };
        if let Some(meta_key) = meta_key {
            let (removed, ids) = {
                let mut pipeline = state
                    .pipeline
                    .lock()
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                let ids: Vec<NodeId> = pipeline
                    .graph
                    .nodes
                    .values()
                    .filter(|n| n.metadata.get(meta_key).map(|v| v == meta_val).unwrap_or(false))
                    .map(|n| n.id)
                    .collect();
                let removed = pipeline
                    .graph
                    .remove_nodes_by_metadata(meta_key, meta_val.as_str());
                (removed, ids)
            };
            removed_graph = removed;
            surreal_targets = ids;

            if row.kind == "codebase" {
                let persisted: Vec<NodeId> = state
                    .surreal_storage
                    .get_user_nodes(user.id, Some("Codebase"), 1)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                    .into_iter()
                    .filter(|r| {
                        r.metadata
                            .get("codebase_scope")
                            .map(|v| v == meta_val)
                            .unwrap_or(false)
                    })
                    .map(|r| r.to_node().id)
                    .collect();
                surreal_targets.extend(persisted);
                surreal_targets.sort_by_key(|id| id.0);
                surreal_targets.dedup_by_key(|id| id.0);
            }

            state
                .surreal_storage
                .delete_node_records(&surreal_targets)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if row.kind == "video" {
                crate::routes::video::remove_stored_video_bundle(meta_val);
            }
        }
    }

    let deleted = delete_user_upload_row(&state.pg_pool, user.id, upload_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if deleted == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "upload row disappeared before delete".to_string(),
        ));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "removed_upload_id": upload_id,
        "removed_graph_nodes": removed_graph,
        "removed_surreal_nodes": surreal_targets.len(),
    })))
}
