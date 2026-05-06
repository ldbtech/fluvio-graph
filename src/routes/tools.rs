//! routes/tools.rs
//!
//! Endpoints:
//!   POST   /tools/detect      — detect action for a user request
//!   POST   /tools/spawn       — start background tool generation job
//!   GET    /tools/jobs/:id    — poll job progress + result
//!   DELETE /tools/jobs/:id    — cancel job + rollback all changes
//!   POST   /tools/approve     — promote generated tool to approved

use std::sync::{Arc, Mutex};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agents::tool_registry::DetectResult;
use crate::agents::tool_spawner::{
    JobManifest, SpawnResult, ToolGenJob,
    ToolGenPhase, ToolGenProgress,
};
use crate::app_state::AppState;

// ── POST /tools/detect ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DetectBody {
    pub request: String,
    pub domain:  Option<String>,
}

#[derive(Serialize)]
pub struct DetectResponse {
    pub action:     String,
    pub tool_name:  Option<String>,
    pub file_name:  Option<String>,
    pub rel_score:  Option<f32>,
    pub similarity: Option<f32>,
}

pub async fn post_tools_detect(
    State(state): State<AppState>,
    Json(body):   Json<DetectBody>,
) -> Result<Json<DetectResponse>, (StatusCode, String)> {
    let request     = body.request.trim().to_string();
    let domain_name = body.domain.as_deref().unwrap_or("architecture");

    if request.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "request is empty".into()));
    }

    let domain = state.tool_spawner.domains.get(domain_name)
        .ok_or((StatusCode::NOT_FOUND, format!("unknown domain: {domain_name}")))?;

    let pipeline = state.pipeline.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let detect = domain.registry.detect(&request, &pipeline);

    let res = match &detect {
        DetectResult::UseExisting { meta, rel_score } => DetectResponse {
            action:     "use_existing".to_string(),
            tool_name:  Some(meta.tool_name.clone()),
            file_name:  Some(meta.file_name.clone()),
            rel_score:  Some(*rel_score),
            similarity: None,
        },
        DetectResult::Extend { meta, rel_score, similarity } => DetectResponse {
            action:     "extend".to_string(),
            tool_name:  Some(meta.tool_name.clone()),
            file_name:  Some(meta.file_name.clone()),
            rel_score:  Some(*rel_score),
            similarity: Some(*similarity),
        },
        DetectResult::Generate { closest_meta, similarity } => DetectResponse {
            action:     "generate".to_string(),
            tool_name:  closest_meta.as_ref().map(|m| m.tool_name.clone()),
            file_name:  closest_meta.as_ref().map(|m| m.file_name.clone()),
            rel_score:  None,
            similarity: Some(*similarity),
        },
    };

    Ok(Json(res))
}

// ── POST /tools/spawn ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SpawnBody {
    pub request: String,
    pub domain:  Option<String>,
}

#[derive(Serialize)]
pub struct SpawnAccepted {
    pub job_id: String,
    pub status: String,
    pub poll:   String,
    pub cancel: String,
}

pub async fn post_tools_spawn(
    State(state): State<AppState>,
    Json(body):   Json<SpawnBody>,
) -> Result<Json<SpawnAccepted>, (StatusCode, String)> {
    let request     = body.request.trim().to_string();
    let domain_name = body.domain.unwrap_or_else(|| "architecture".to_string());

    if request.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "request is empty".into()));
    }

    if !state.tool_spawner.domains.contains_key(&domain_name) {
        return Err((StatusCode::NOT_FOUND, format!("unknown domain: {domain_name}")));
    }

    let job_id   = Uuid::new_v4().to_string();
    let progress = Arc::new(Mutex::new(ToolGenProgress::new(&job_id)));
    let result   = Arc::new(Mutex::new(None::<SpawnResult>));
    let manifest = Arc::new(Mutex::new(JobManifest::new(&job_id)));
    let cancel   = tokio_util::sync::CancellationToken::new();

    // Register job
    {
        let mut store = state.job_store.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        store.insert(job_id.clone(), ToolGenJob {
            job_id:   job_id.clone(),
            request:  request.clone(),
            domain:   domain_name.clone(),
            progress: progress.clone(),
            result:   result.clone(),
            manifest: manifest.clone(),
            cancel:   cancel.clone(),
        });
    }

    // Clones for background task
    let spawner      = state.tool_spawner.clone();
    let pipeline_arc = state.pipeline.clone();
    let job_id_bg    = job_id.clone();

    tokio::spawn(async move {
        // ── Phase: Detecting ─────────────────────────────────────────────────
        set_progress(&progress, ToolGenPhase::Detecting, 5, "Detecting best action...");

        if cancel.is_cancelled() {
            set_progress(&progress, ToolGenPhase::Failed, 0, "Cancelled");
            return;
        }

        // ── Phase: Writing spec ───────────────────────────────────────────────
        set_progress(&progress, ToolGenPhase::WritingSpec, 15, "Writing tool specification...");

        if cancel.is_cancelled() {
            set_progress(&progress, ToolGenPhase::Failed, 0, "Cancelled");
            return;
        }

        // ── Phase: Generating ─────────────────────────────────────────────────
        set_progress(&progress, ToolGenPhase::Generating, 40, "Generating tool code...");

        // Clone manifest for use in spawn
        let mut job_manifest = match manifest.lock() {
            Ok(m)  => m.clone(),
            Err(_) => JobManifest::new(&job_id_bg),
        };

        // ── Run spawn — use block_in_place so std::sync::MutexGuard is safe ──
        // block_in_place moves blocking work off the async executor thread.
        // block_on drives the async spawn() to completion synchronously.
        // This means the MutexGuard never crosses an actual await boundary.
        let handle = tokio::runtime::Handle::current();

        let spawn_result: anyhow::Result<SpawnResult> = tokio::task::block_in_place(|| {
            let mut pipeline = match pipeline_arc.lock() {
                Ok(p)  => p,
                Err(e) => return Err(anyhow::anyhow!("pipeline lock failed: {e}")),
            };

            handle.block_on(
                spawner.spawn(&request, &domain_name, &mut pipeline, &mut job_manifest)
            )
        });

        // ── Phase: Ingesting ──────────────────────────────────────────────────
        set_progress(&progress, ToolGenPhase::Ingesting, 85, "Registering in knowledge graph...");

        match spawn_result {
            Ok(res) => {
                set_progress(&progress, ToolGenPhase::Done, 100, "Tool ready for approval");
                if let Ok(mut r) = result.lock() {
                    *r = Some(res);
                }
            }
            Err(e) => {
                if let Ok(mut p) = progress.lock() {
                    p.fail(&e.to_string());
                }
                // Rollback on error
                if let Ok(m) = manifest.lock() {
                    if let Ok(mut pipeline) = pipeline_arc.lock() {
                        m.rollback(&mut pipeline.graph);
                    }
                }
            }
        }
    });

    Ok(Json(SpawnAccepted {
        job_id:  job_id.clone(),
        status:  "accepted".to_string(),
        poll:    format!("/tools/jobs/{job_id}"),
        cancel:  format!("/tools/jobs/{job_id}"),
    }))
}

// ── GET /tools/jobs/:id ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct JobStatusResponse {
    pub job_id:  String,
    pub phase:   String,
    pub percent: u8,
    pub message: String,
    pub error:   Option<String>,
    pub done:    bool,
    pub result:  Option<SpawnResult>,
}

pub async fn get_tools_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<JobStatusResponse>, (StatusCode, String)> {
    let store = state.job_store.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job = store.get(&job_id)
        .ok_or((StatusCode::NOT_FOUND, format!("job '{job_id}' not found")))?;

    let progress = job.progress.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let done = matches!(progress.phase, ToolGenPhase::Done | ToolGenPhase::Failed);

    let result = if done {
        job.result.lock().ok().and_then(|r| r.clone())
    } else {
        None
    };

    Ok(Json(JobStatusResponse {
        job_id,
        phase:   format!("{:?}", progress.phase).to_lowercase(),
        percent: progress.percent,
        message: progress.message.clone(),
        error:   progress.error.clone(),
        done,
        result,
    }))
}

// ── DELETE /tools/jobs/:id ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct CancelResponse {
    pub job_id:         String,
    pub cancelled:      bool,
    pub rolled_back:    bool,
    pub deleted_files:  Vec<String>,
    pub restored_files: Vec<String>,
    pub removed_nodes:  Vec<String>,
}

pub async fn delete_tools_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<CancelResponse>, (StatusCode, String)> {
    let mut store = state.job_store.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let job = store.remove(&job_id)
        .ok_or((StatusCode::NOT_FOUND, format!("job '{job_id}' not found")))?;

    // Signal the background task to stop
    job.cancel.cancel();

    let manifest = job.manifest.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let deleted_files: Vec<String> = manifest.created_files.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let restored_files: Vec<String> = manifest.modified_files.iter()
        .map(|s| s.path.to_string_lossy().to_string())
        .collect();

    let removed_nodes = manifest.created_nodes.clone();

    {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        manifest.rollback(&mut pipeline.graph);
    }

    Ok(Json(CancelResponse {
        job_id,
        cancelled:      true,
        rolled_back:    true,
        deleted_files,
        restored_files,
        removed_nodes,
    }))
}

// ── POST /tools/approve ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ApproveBody {
    pub file_name: String,
    pub domain:    Option<String>,
    pub job_id:    Option<String>,
}

#[derive(Serialize)]
pub struct ApproveResponse {
    pub file_name: String,
    pub file_path: String,
    pub approved:  bool,
}

pub async fn post_tools_approve(
    State(state): State<AppState>,
    Json(body):   Json<ApproveBody>,
) -> Result<Json<ApproveResponse>, (StatusCode, String)> {
    let file_name   = body.file_name.trim().to_string();
    let domain_name = body.domain.as_deref().unwrap_or("architecture");

    if file_name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "file_name is empty".into()));
    }

    let mut pipeline = state.pipeline.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let path = state.tool_spawner
        .approve(&file_name, domain_name, &mut pipeline)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    (state.presist)(&pipeline.graph)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    drop(pipeline);

    // Clean up completed job
    if let Some(job_id) = &body.job_id {
        if let Ok(mut store) = state.job_store.lock() {
            store.remove(job_id);
        }
    }

    Ok(Json(ApproveResponse {
        file_name,
        file_path: path.to_string_lossy().to_string(),
        approved:  true,
    }))
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn set_progress(
    progress: &Arc<Mutex<ToolGenProgress>>,
    phase:    ToolGenPhase,
    percent:  u8,
    message:  &str,
) {
    if let Ok(mut p) = progress.lock() {
        p.update(phase, percent, message);
    }
}