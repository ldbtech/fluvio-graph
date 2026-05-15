//! Contribution approval types.
//!
//! Every mutation a `Contributor` makes to the shared graph first lands in
//! the approval queue (`ApprovalStatus::Pending`). The group owner reviews
//! and either promotes it to `Approved` (node becomes live) or `Rejected`
//! (node is soft-deleted from SurrealDB).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collab::group::GroupId;

// ── NodeType ──────────────────────────────────────────────────────────────────

/// What kind of graph object the contribution adds.
/// Stored as a string in Postgres `approval_queue.node_type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    /// A knowledge node from a PDF, document, or raw text.
    Knowledge,
    /// A tool definition node.
    Tool,
    /// An agent definition node.
    Agent,
    /// A PDF upload staged for graph extraction.
    Pdf,
    /// A connector configuration node.
    Connector,
}

impl std::fmt::Display for ContributionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ContributionKind::Knowledge  => "knowledge",
            ContributionKind::Tool       => "tool",
            ContributionKind::Agent      => "agent",
            ContributionKind::Pdf        => "pdf",
            ContributionKind::Connector  => "connector",
        };
        write!(f, "{s}")
    }
}

// ── ApprovalStatus ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    /// Written to SurrealDB with `status = "pending"`, not yet visible to others.
    Pending,
    /// Owner approved — node promoted to live, visible to all group members.
    Approved,
    /// Owner rejected — node soft-deleted from SurrealDB.
    Rejected,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ApprovalStatus::Pending  => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
        };
        write!(f, "{s}")
    }
}

// ── Contribution ──────────────────────────────────────────────────────────────

/// A pending contribution from a group member awaiting owner review.
/// Maps to Postgres `approval_queue` table.
/// The `surreal_node_id` field links back to the SurrealDB node that was written
/// with `status = "pending"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub id:               Uuid,
    pub group_id:         GroupId,
    /// Postgres `users.id` of the contributor.
    pub contributed_by:   Uuid,
    pub kind:             ContributionKind,
    /// The SurrealDB node ID written with status=pending.
    pub surreal_node_id:  String,
    pub status:           ApprovalStatus,
    /// Postgres `users.id` of the reviewer (owner).
    pub reviewed_by:      Option<Uuid>,
    pub review_note:      Option<String>,
    pub created_at:       DateTime<Utc>,
    pub reviewed_at:      Option<DateTime<Utc>>,
}

impl Contribution {
    pub fn new(
        group_id:        GroupId,
        contributed_by:  Uuid,
        kind:            ContributionKind,
        surreal_node_id: impl Into<String>,
    ) -> Self {
        Self {
            id:              Uuid::new_v4(),
            group_id,
            contributed_by,
            kind,
            surreal_node_id: surreal_node_id.into(),
            status:          ApprovalStatus::Pending,
            reviewed_by:     None,
            review_note:     None,
            created_at:      Utc::now(),
            reviewed_at:     None,
        }
    }

    pub fn is_pending(&self) -> bool {
        self.status == ApprovalStatus::Pending
    }
}