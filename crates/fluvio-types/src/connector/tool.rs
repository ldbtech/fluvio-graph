//! Tool types — shared between fluvio-tool-builder and fluvio-collab.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::collab::group::GroupId;
use crate::connector::connector::ConnectorId;
use crate::graph::ids::NodeId;

// ── ToolStatus ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStatus {
    /// Written locally, not yet submitted to the group.
    Local,
    /// Submitted, awaiting owner approval.
    Pending,
    /// Owner approved — tool is live and runnable.
    Approved,
    /// Owner rejected.
    Rejected,
    /// Approved and actively being used.
    Active,
}

// ── ToolDraft ─────────────────────────────────────────────────────────────────

/// A tool description authored in the Agent Studio UI.
/// Drafts are local-only until submitted as a `Contribution`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDraft {
    pub id:               Uuid,
    pub agent_id:         Option<Uuid>,
    pub name:             String,
    /// Prose specification: what it reads from the graph, what connectors
    /// it calls, what it outputs, and where humans must confirm.
    pub description:      String,
    /// Which connectors this tool is permitted to call.
    pub connector_scope:  Vec<ConnectorId>,
    /// Graph nodes this tool reads as grounding knowledge.
    pub reads_from_nodes: Vec<NodeId>,
    pub status:           ToolStatus,
    pub created_at:       DateTime<Utc>,
    pub updated_at:       DateTime<Utc>,
}

impl ToolDraft {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id:               Uuid::new_v4(),
            agent_id:         None,
            name:             name.into(),
            description:      description.into(),
            connector_scope:  Vec::new(),
            reads_from_nodes: Vec::new(),
            status:           ToolStatus::Local,
            created_at:       now,
            updated_at:       now,
        }
    }
}

// ── ToolManifest ──────────────────────────────────────────────────────────────

/// The full manifest written to disk by `fluvio-tool-builder`.
/// Lives at `~/fluvio-workspace/tools/<slug>/manifest.toml`.
/// Mirrors the TOML format the tool-builder reads and writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub id:               Uuid,
    pub name:             String,
    pub description:      String,
    pub version:          String,
    /// Postgres `users.id` of the creator.
    pub created_by:       Uuid,
    pub group_id:         GroupId,
    pub status:           ToolStatus,
    pub connector_scope:  Vec<ConnectorId>,
    pub reads_from_nodes: Vec<NodeId>,
    /// Input parameters schema (field name → JSON type string).
    pub inputs:           HashMap<String, String>,
    /// Output schema description.
    pub outputs:          HashMap<String, String>,
    /// Runtime config.
    pub language:         String,
    pub entrypoint:       String,
    pub sandbox:          bool,
    pub timeout_sec:      u32,
    pub created_at:       DateTime<Utc>,
}

// ── ToolRunResult ─────────────────────────────────────────────────────────────

/// The result of a tool execution emitted by `fluvio-tool-builder`.
/// Sent to `fluvio-ingestion` which turns it into graph nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRunResult {
    pub tool_id:     Uuid,
    pub run_id:      Uuid,
    pub group_id:    GroupId,
    pub success:     bool,
    pub output:      serde_json::Value,
    pub error:       Option<String>,
    pub duration_ms: u64,
    pub ran_at:      DateTime<Utc>,
}

impl ToolRunResult {
    pub fn success(tool_id: Uuid, group_id: GroupId, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            tool_id,
            run_id:      Uuid::new_v4(),
            group_id,
            success:     true,
            output,
            error:       None,
            duration_ms,
            ran_at:      Utc::now(),
        }
    }

    pub fn failure(tool_id: Uuid, group_id: GroupId, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            tool_id,
            run_id:      Uuid::new_v4(),
            group_id,
            success:     false,
            output:      serde_json::Value::Null,
            error:       Some(error.into()),
            duration_ms,
            ran_at:      Utc::now(),
        }
    }
}