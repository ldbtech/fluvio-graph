//! routes/video.rs
//!
//! Endpoints:
//!   POST /ingest/video        — upload video, detect scenes, spawn LLaVA background tasks
//!   GET  /video/{id}          — get video node + all scene nodes
//!   GET  /video/{id}/scenes   — list scenes with understanding status
//!   GET  /video/{id}/status   — LLaVA/Ollama progress (complete, failed, pending, processed %)

use std::path::PathBuf;
use std::sync::Arc;
use axum::{
    Json,
    extract::{FromRequest, Multipart, Path, Request, State},
    http::StatusCode,
};
use serde::Serialize;
use uuid::Uuid;

use crate::ingestion_registry::videos::{
    extractor::{SceneDetectionConfig, detect_scenes, extract_metadata},
    normalizer::{
        mark_scene_understanding_failed, scenes_to_chunks, update_scene_understanding, video_uri,
    },
    frame::extract_frame_bytes,
    vision::{VisionConfig, describe_scene},
};
use crate::app_state::AppState;
use crate::database::user_uploads::insert_user_upload;
use crate::storage::surreal::SurrealStorage;
use crate::authentication::upload_user_id_from_headers;
use crate::database::users::{get_user_by_id, user_physical_scope};

// ── Storage path ──────────────────────────────────────────────────────────────

/// Root directory for uploaded videos.
const VIDEO_STORE: &str = "fluvio_videos";

fn video_dir(video_id: &str) -> PathBuf {
    PathBuf::from(VIDEO_STORE).join(video_id)
}

fn video_file_path(video_id: &str) -> PathBuf {
    video_dir(video_id).join("original.mp4")
}

/// Best-effort removal of on-disk assets for an uploaded clip (used when the user deletes a library entry).
pub fn remove_stored_video_bundle(video_id: &str) {
    let dir = video_dir(video_id);
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            tracing::warn!("[Video] could not remove store dir {}: {e}", dir.display());
        }
    }
}

// ── POST /ingest/video ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IngestVideoResponse {
    pub video_id:    String,
    pub duration:    f64,
    pub fps:         f64,
    pub resolution:  String,
    pub codec:       String,
    pub scenes:      usize,
    pub nodes:       usize,
    pub edges:       usize,
    pub has_audio:   bool,
    pub status:      String,
}

pub async fn post_ingest_video(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<IngestVideoResponse>, (StatusCode, String)> {
    let uid = upload_user_id_from_headers(req.headers()).ok_or((
        StatusCode::UNAUTHORIZED,
        "missing upload authentication".to_string(),
    ))?;

    let user = get_user_by_id(&state.pg_pool, uid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "session user not found".to_string(),
        ))?;

    let mut multipart = Multipart::from_request(req, &state)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // ── 1. Receive file from multipart ────────────────────────────────────────
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut client_file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {e}")))? {

        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "video" {
            client_file_name = field
                .file_name()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty());
            let bytes = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("read error: {e}")))?;
            file_bytes = Some(bytes.to_vec());
            break;
        }
    }

    let bytes = file_bytes
        .ok_or((StatusCode::BAD_REQUEST, "no file field in multipart".to_string()))?;

    let upload_label = client_file_name.unwrap_or_else(|| "video.mp4".to_string());

    if bytes.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty file".to_string()));
    }

    // ── 2. Save to disk ───────────────────────────────────────────────────────
    let video_id  = Uuid::new_v4().to_string();
    let dir       = video_dir(&video_id);
    let file_path = video_file_path(&video_id);

    std::fs::create_dir_all(&dir)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("mkdir: {e}")))?;

    std::fs::write(&file_path, &bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("write: {e}")))?;

    tracing::info!("[Video] saved {} bytes → {}", bytes.len(), file_path.display());

    // ── 3. Extract metadata ───────────────────────────────────────────────────
    let meta = extract_metadata(&file_path, &video_id)
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, format!("metadata: {e}")))?;

    tracing::info!(
        "[Video] {video_id} — {:.1}s {:.0}fps {}x{} {}",
        meta.duration, meta.fps, meta.width, meta.height, meta.codec
    );

    // ── 4. Detect scenes ──────────────────────────────────────────────────────
    let cfg    = SceneDetectionConfig::default();
    let scenes = detect_scenes(&file_path, &cfg, &meta)
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, format!("scene detection: {e}")))?;

    tracing::info!("[Video] {video_id} — {} scenes detected", scenes.len());

    // ── 5. Ingest into graph ──────────────────────────────────────────────────
    let chunks = scenes_to_chunks(&meta, &scenes, 0);

    let (nodes, edges) = {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        pipeline.ingest_normalized_chunks(&chunks)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("ingest: {e:?}")))?
    };

    tracing::info!("[Video] {video_id} — {nodes} nodes, {edges} edges ingested");

    let vuri             = video_uri(&video_id);
    let scope            = user_physical_scope(&user);
    let owner_str        = user.id.to_string();
    let surreal_snapshot = {
        let mut pipeline = state
            .pipeline
            .lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let mut subgraph_ids = Vec::new();
        for n in pipeline.graph.nodes.values_mut() {
            if n.metadata.get("video_id").map(|v| v.as_str()) != Some(video_id.as_str()) {
                continue;
            }
            n.metadata.insert("owner_id".into(), owner_str.clone());
            n.metadata.insert("owner_physical_id".into(), scope.clone());
            n.metadata.insert("zone".into(), "1".into());
            subgraph_ids.push(n.id);
            if n.source_uri == vuri {
                n.metadata.insert("dashboard_doc_anchor".into(), "1".into());
                n.metadata.entry("kind".into()).or_insert_with(|| "video".into());
                n.metadata
                    .entry("title".into())
                    .or_insert_with(|| format!("Video {}", &video_id[..video_id.len().min(12)]));
            }
        }
        pipeline.graph.subgraph_closed(subgraph_ids.into_iter())
    };

    if !surreal_snapshot.nodes.is_empty() {
        if let Err(e) = state
            .surreal_storage
            .save_graph(user.id, &surreal_snapshot, 1)
            .await
        {
            tracing::warn!("[SurrealDB] video ingest persist skipped: {e}");
        }
    }

    let sub_nodes = surreal_snapshot.nodes.len() as i32;
    let sub_edges: i32 = surreal_snapshot
        .adj
        .values()
        .map(|e| e.len() as i32)
        .sum();
    if let Err(e) = insert_user_upload(
        &state.pg_pool,
        user.id,
        "video",
        &upload_label,
        Some(video_id.as_str()),
        sub_nodes,
        sub_edges,
    )
    .await
    {
        tracing::warn!("[DB] user_uploads insert skipped: {e}");
    }

    // ── 6. Spawn background LLaVA tasks per scene ─────────────────────────────
    let pipeline_arc  = state.pipeline.clone();
    let surreal_arc: Arc<SurrealStorage> = state.surreal_storage.clone();
    let owner_uuid    = user.id;
    let vision_config = VisionConfig::from_env();
    let vid_id        = video_id.clone();
    let fp            = file_path.clone();
    let scene_list    = scenes.clone();

    tokio::spawn(async move {
        tracing::info!(
            "[Vision] starting background understanding for {} scenes",
            scene_list.len()
        );

        for scene in &scene_list {
            // Extract frame at sample_time
            let frame_result = tokio::task::spawn_blocking({
                let fp2    = fp.clone();
                let sample = scene.sample_time;
                move || extract_frame_bytes(&fp2, sample)
            }).await;

            let frame_bytes = match frame_result {
                Ok(Ok(b))  => b,
                Ok(Err(e)) => {
                    tracing::warn!(
                        "[Vision] scene {} frame extract failed: {e}",
                        scene.index
                    );
                    if let Ok(mut pipeline) = pipeline_arc.lock() {
                        mark_scene_understanding_failed(
                            &vid_id,
                            scene.index,
                            &format!("frame extract: {e}"),
                            &mut pipeline.graph,
                        );
                    }
                    continue;
                }
                Err(e) => {
                    tracing::warn!("[Vision] scene {} spawn failed: {e}", scene.index);
                    if let Ok(mut pipeline) = pipeline_arc.lock() {
                        mark_scene_understanding_failed(
                            &vid_id,
                            scene.index,
                            &format!("spawn: {e}"),
                            &mut pipeline.graph,
                        );
                    }
                    continue;
                }
            };

            // Send to LLaVA
            let description = match describe_scene(&frame_bytes, &vision_config).await {
                Ok(d)  => d,
                Err(e) => {
                    tracing::warn!(
                        "[Vision] scene {} LLaVA failed: {e}",
                        scene.index
                    );
                    if let Ok(mut pipeline) = pipeline_arc.lock() {
                        mark_scene_understanding_failed(
                            &vid_id,
                            scene.index,
                            &format!("ollama/llava: {e}"),
                            &mut pipeline.graph,
                        );
                    }
                    continue;
                }
            };

            // Update graph node
            if let Ok(mut pipeline) = pipeline_arc.lock() {
                update_scene_understanding(
                    &vid_id,
                    scene.index,
                    &description,
                    &mut pipeline.graph,
                );
            }

            tracing::info!(
                "[Vision] scene {} complete: {}…",
                scene.index,
                &description[..description.len().min(60)]
            );
        }

        let vision_subgraph = (|| {
            let Ok(pipeline) = pipeline_arc.lock() else {
                return None;
            };
            let vuri = video_uri(&vid_id);
            let ids: Vec<_> = pipeline
                .graph
                .nodes
                .values()
                .filter(|n| {
                    n.metadata
                        .get("video_id")
                        .map(|v| v == vid_id.as_str())
                        .unwrap_or(false)
                        || n.source_uri == vuri
                })
                .map(|n| n.id)
                .collect();
            if ids.is_empty() {
                None
            } else {
                Some(pipeline.graph.subgraph_closed(ids.into_iter()))
            }
        })();

        if let Some(snap) = vision_subgraph {
            if !snap.nodes.is_empty() {
                if let Err(e) = surreal_arc.save_graph(owner_uuid, &snap, 1).await {
                    tracing::warn!("[SurrealDB] video vision persist skipped: {e}");
                }
            }
        }

        tracing::info!("[Vision] all scenes processed for {vid_id}");
    });

    Ok(Json(IngestVideoResponse {
        video_id,
        duration:   meta.duration,
        fps:        meta.fps,
        resolution: format!("{}x{}", meta.width, meta.height),
        codec:      meta.codec,
        scenes:     scenes.len(),
        nodes,
        edges,
        has_audio:  meta.has_audio,
        status:     "ingested — understanding in progress".to_string(),
    }))
}

// ── GET /video/{id} ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct VideoResponse {
    pub video_id:    String,
    pub duration:    f64,
    pub fps:         f64,
    pub resolution:  String,
    pub codec:       String,
    pub scene_count: usize,
    pub has_audio:   bool,
    pub scenes:      Vec<SceneResponse>,
}

#[derive(Serialize)]
pub struct SceneResponse {
    pub scene_index:   usize,
    pub time_start:    f64,
    pub time_end:      f64,
    pub duration:      f64,
    pub sample_time:   f64,
    pub score:         f64,
    pub understanding: String,
    /// Populated when `understanding` is `failed` (Ollama unreachable, timeout, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub understanding_error: Option<String>,
    pub description:   Option<String>,
    pub source_uri:    String,
}

pub async fn get_video(
    State(state): State<AppState>,
    Path(video_id): Path<String>,
) -> Result<Json<VideoResponse>, (StatusCode, String)> {
    let pipeline = state.pipeline.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let graph = &pipeline.graph;

    // Find video node
    let video_node = graph.nodes.values()
        .find(|n| n.source_uri == video_uri(&video_id))
        .ok_or((StatusCode::NOT_FOUND, format!("video '{video_id}' not found")))?;

    let get = |k: &str| video_node.metadata.get(k).cloned().unwrap_or_default();

    // Collect scene nodes ordered by index
    let mut scene_responses: Vec<SceneResponse> = graph.nodes.values()
        .filter(|n| {
            n.metadata.get("kind").map(|k| k == "scene").unwrap_or(false)
                && n.metadata.get("video_id").map(|v| v == &video_id).unwrap_or(false)
        })
        .map(|n| {
            let get_n = |k: &str| n.metadata.get(k).cloned().unwrap_or_default();
            let parse  = |k: &str| get_n(k).parse::<f64>().unwrap_or(0.0);
            let idx    = get_n("scene_index").parse::<usize>().unwrap_or(0);

            let err = get_n("understanding_error");
            let understanding_error = if err.is_empty() {
                None
            } else {
                Some(err)
            };

            SceneResponse {
                scene_index:   idx,
                time_start:    parse("time_start"),
                time_end:      parse("time_end"),
                duration:      parse("duration"),
                sample_time:   parse("sample_time"),
                score:         parse("score"),
                understanding: get_n("understanding"),
                understanding_error,
                description:   if n.source_text.contains("pending")
                    || n.source_text.is_empty()
                    || n.metadata.get("understanding").map(|u| u == "failed").unwrap_or(false)
                {
                    None
                } else {
                    Some(n.source_text.clone())
                },
                source_uri:    n.source_uri.clone(),
            }
        })
        .collect();

    scene_responses.sort_by_key(|s| s.scene_index);

    let fps        = get("fps").parse::<f64>().unwrap_or(0.0);
    let duration   = get("duration").parse::<f64>().unwrap_or(0.0);
    let width      = get("width");
    let height     = get("height");
    let scene_count = get("scene_count").parse::<usize>().unwrap_or(0);

    Ok(Json(VideoResponse {
        video_id,
        duration,
        fps,
        resolution:  format!("{width}x{height}"),
        codec:       get("codec"),
        scene_count,
        has_audio:   get("has_audio") == "true",
        scenes:      scene_responses,
    }))
}

// ── GET /video/{id}/scenes ────────────────────────────────────────────────────

pub async fn get_video_scenes(
    State(state): State<AppState>,
    Path(video_id): Path<String>,
) -> Result<Json<Vec<SceneResponse>>, (StatusCode, String)> {
    let res = get_video(State(state), Path(video_id)).await?;
    Ok(Json(res.0.scenes))
}

// ── GET /video/{id}/status ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct VideoStatusResponse {
    pub video_id:   String,
    pub total:      usize,
    /// Scenes with a successful LLaVA description (`understanding: complete`).
    pub complete:   usize,
    /// Scenes that finished with an error (`understanding: failed`).
    pub failed:     usize,
    /// Still waiting on or running local vision.
    pub pending:    usize,
    /// Share of scenes with a successful description (`complete * 100 / total`).
    pub percent:    u8,
    /// Share of scenes that finished processing, success or failure (`(complete+failed)*100/total`).
    pub processed_percent: u8,
    pub done:       bool,
}

pub async fn get_video_status(
    State(state): State<AppState>,
    Path(video_id): Path<String>,
) -> Result<Json<VideoStatusResponse>, (StatusCode, String)> {
    let pipeline = state.pipeline.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let graph = &pipeline.graph;

    let scene_nodes: Vec<_> = graph.nodes.values()
        .filter(|n| {
            n.metadata.get("kind").map(|k| k == "scene").unwrap_or(false)
                && n.metadata.get("video_id").map(|v| v == &video_id).unwrap_or(false)
        })
        .collect();

    if scene_nodes.is_empty() {
        return Err((StatusCode::NOT_FOUND, format!("video '{video_id}' not found")));
    }

    let total    = scene_nodes.len();
    let complete = scene_nodes
        .iter()
        .filter(|n| n.metadata.get("understanding").map(|u| u == "complete").unwrap_or(false))
        .count();
    let failed = scene_nodes
        .iter()
        .filter(|n| n.metadata.get("understanding").map(|u| u == "failed").unwrap_or(false))
        .count();
    let pending = total.saturating_sub(complete).saturating_sub(failed);
    let percent = if total == 0 {
        0
    } else {
        (complete * 100 / total) as u8
    };
    let processed_percent = if total == 0 {
        0
    } else {
        ((complete + failed) * 100 / total) as u8
    };

    Ok(Json(VideoStatusResponse {
        video_id,
        total,
        complete,
        failed,
        pending,
        percent,
        processed_percent,
        done: pending == 0,
    }))
}