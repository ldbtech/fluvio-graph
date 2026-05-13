//! routes/rules.rs
//!
//! Endpoints:
//!   POST /rules/link                  — link PDF rule nodes to codebase nodes
//!   POST /agents/security/deploy      — deploy security agent as background job
//!   GET  /agents/security/:id/status  — poll agent progress
//!   GET  /agents/security/:id/result  — get final result when done

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::graph::fluvio_graph::FluvioGraph;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent_jobs::AgentJobEntry;
use crate::app_state::AppState;
use crate::ingestion_registry::documents::rule_linker::{
    linker::{
        LinkConfig, LinkResult, NodeView,
        build_result, link_by_similarity, link_hybrid, partition_nodes,
    },
    security_agent::{
        AgentProgress, SecurityAgentConfig, SecurityAgentProgress,
        SecurityAgentResult, run_agent,
    },
};

// ── POST /rules/link ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RulesLinkBody {
    /// Optional: only use PDF nodes from this document_id.
    #[serde(default)]
    pub document_id: Option<String>,

    /// Optional: only match against codebase nodes under this path prefix.
    #[serde(default)]
    pub code_path_filter: Option<String>,

    /// Similarity threshold (default 0.65).
    #[serde(default = "default_threshold")]
    pub similarity_threshold: f32,

    /// Max PDF rules to match per code node.
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// Use LLM for final classification (Option C).
    /// If false, similarity + heuristics only (Option A).
    #[serde(default = "default_use_llm")]
    pub use_llm: bool,
}

fn default_threshold() -> f32  { 0.65 }
fn default_top_k()     -> usize { 5 }
fn default_use_llm()   -> bool  { true }

pub async fn post_rules_link(
    State(state): State<AppState>,
    Json(body):   Json<RulesLinkBody>,
) -> Result<Json<LinkResult>, (StatusCode, String)> {
    // Snapshot all nodes from the graph.
    let node_views: Vec<NodeView> = {
        let pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        snapshot_node_views(&pipeline.graph)
    };

    if node_views.is_empty() {
        return Err((StatusCode::NOT_FOUND,
            "no nodes in graph — ingest a repo and a PDF first".to_string()));
    }

    let (rule_nodes, code_nodes) = partition_nodes(&node_views);

    if rule_nodes.is_empty() {
        return Err((StatusCode::NOT_FOUND,
            "no PDF rule nodes in graph — upload a security PDF first".to_string()));
    }
    if code_nodes.is_empty() {
        return Err((StatusCode::NOT_FOUND,
            "no codebase nodes in graph — ingest a repo first".to_string()));
    }

    // Determine document_id and filename from the first matching rule node.
    let document_id = body.document_id.clone()
        .or_else(|| rule_nodes.first().map(|n| n.doc_id.clone()))
        .unwrap_or_else(|| "unknown".to_string());
    let filename = rule_nodes.first()
        .map(|n| n.filename.clone())
        .unwrap_or_else(|| "unknown.pdf".to_string());

    let config = LinkConfig {
        similarity_threshold: body.similarity_threshold,
        top_k:                body.top_k,
        use_llm:              body.use_llm,
        api_key:              if body.use_llm { Some(state.api_key.clone()) } else { None },
        document_id_filter:   body.document_id.clone(),
        code_path_filter:     body.code_path_filter.clone(),
    };

    // Run linker — Option C (hybrid) or Option A (similarity only).
    let matches = if body.use_llm {
        link_hybrid(&rule_nodes, &code_nodes, &config).await
    } else {
        link_by_similarity(&rule_nodes, &code_nodes, &config)
    };

    // Write edges into the graph.
    {
        let mut pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for m in &matches {
            // Find node IDs from URIs.
            let from_id = pipeline.graph.nodes.values()
                .find(|n| n.source_uri == m.code_uri)
                .map(|n| n.id);
            let to_id = pipeline.graph.nodes.values()
                .find(|n| n.source_uri == m.rule_uri)
                .map(|n| n.id);

            if let (Some(from), Some(to)) = (from_id, to_id) {
                use crate::graph::structs::{Edge, EdgeId};
                let edge = Edge {
                    id:                       EdgeId::new(),
                    from,
                    to,
                    token:                    ((1.0 - m.confidence) * 1000.0) as i32,
                    relationship_probability: m.confidence,
                    label:                    m.edge_kind.as_label().to_string(),
                    metadata:                 HashMap::new(),
                };
                let _ = pipeline.graph.insert_edge(edge);
            }
        }
    }

    let result = build_result(&document_id, &filename, matches);
    Ok(Json(result))
}

// ── POST /agents/security/deploy ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SecurityDeployBody {
    /// Only analyze codebase nodes under this path prefix.
    #[serde(default)]
    pub scope: Option<String>,

    /// Only use PDF nodes from these document IDs (empty = all PDFs).
    #[serde(default)]
    pub pdf_document_ids: Vec<String>,

    /// Minimum similarity threshold (default 0.55).
    #[serde(default = "default_agent_threshold")]
    pub similarity_threshold: f32,

    /// Max PDF rules per code file (default 5).
    #[serde(default = "default_agent_top_k")]
    pub top_k_rules: usize,

    /// Max files to analyze (default 100).
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_agent_threshold() -> f32  { 0.55 }
fn default_agent_top_k()     -> usize { 5 }
fn default_max_files()       -> usize { 100 }

#[derive(Serialize)]
pub struct SecurityDeployResponse {
    pub agent_id: String,
    pub status:   String,
    pub poll:     String,
    pub result:   String,
}

pub async fn post_security_deploy(
    State(state): State<AppState>,
    Json(body):   Json<SecurityDeployBody>,
) -> Result<Json<SecurityDeployResponse>, (StatusCode, String)> {
    let agent_id = Uuid::new_v4().to_string();

    let config = SecurityAgentConfig {
        scope:                body.scope,
        pdf_document_ids:     body.pdf_document_ids,
        similarity_threshold: body.similarity_threshold,
        top_k_rules:          body.top_k_rules,
        max_files:            body.max_files,
    };

    let progress = Arc::new(SecurityAgentProgress::new(&agent_id));
    let result_store: Arc<Mutex<Option<SecurityAgentResult>>> = Arc::new(Mutex::new(None));

    // Store in agent registry.
    {
        let mut store = state.agent_store.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        store.insert(agent_id.clone(), AgentJobEntry {
            progress: progress.clone(),
            result:   result_store.clone(),
        });
    }

    // Snapshot the graph for the background task.
    let graph_arc = Arc::new(Mutex::new({
        let pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        pipeline.graph.clone()
    }));

    let embed_ctx  = {
        let pipeline = state.pipeline.lock()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        pipeline.embed_ctx.clone()
    };

    let api_key    = state.api_key.clone();
    let agent_id_clone = agent_id.clone();

    // Spawn background task.
    tokio::spawn(async move {
        let result = run_agent(
            agent_id_clone,
            config,
            api_key,
            graph_arc,
            embed_ctx,
            progress,
        ).await;

        *result_store.lock().unwrap() = Some(result);
    });

    Ok(Json(SecurityDeployResponse {
        agent_id: agent_id.clone(),
        status:   "accepted".to_string(),
        poll:     format!("/agents/security/{agent_id}/status"),
        result:   format!("/agents/security/{agent_id}/result"),
    }))
}

// ── GET /agents/security/:id/status ──────────────────────────────────────────

pub async fn get_security_status(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentProgress>, (StatusCode, String)> {
    let store = state.agent_store.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let entry = store.get(&agent_id)
        .ok_or((StatusCode::NOT_FOUND,
            format!("agent '{agent_id}' not found")))?;

    Ok(Json(entry.progress.snapshot()))
}

// ── GET /agents/security/:id/result ──────────────────────────────────────────

pub async fn get_security_result(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<SecurityAgentResult>, (StatusCode, String)> {
    let store = state.agent_store.lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let entry = store.get(&agent_id)
        .ok_or((StatusCode::NOT_FOUND,
            format!("agent '{agent_id}' not found")))?;

    let result = entry.result.lock().unwrap();
    match result.as_ref() {
        Some(r) => Ok(Json(r.clone())),
        None => Err((StatusCode::ACCEPTED,
            "agent is still running — poll /status first".to_string())),
    }
}

// ── Graph snapshot helper ─────────────────────────────────────────────────────

/// Snapshot all nodes from the graph into `NodeView`s for the linker.
fn snapshot_node_views(
    graph: &crate::graph::structs::DomainGraph,
) -> Vec<NodeView> {
    graph.nodes.values().map(|n| NodeView {
        id:         n.id.to_string(),
        uri:        n.source_uri.clone(),
        text:       n.source_text.clone(),
        source:     n.metadata.get("source").cloned().unwrap_or_default(),
        filename:   n.metadata.get("filename").cloned().unwrap_or_default(),
        doc_id:     n.metadata.get("document_id").cloned().unwrap_or_default(),
        path:       n.metadata.get("path").cloned().unwrap_or_default(),
        symbol:     n.metadata.get("name").cloned(),
        embeddings: n.embeddings.clone(),
    }).collect()
}