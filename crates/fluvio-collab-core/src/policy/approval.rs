//! Approval routing policy — pure functions.
//!
//! Determines whether a contribution needs owner review
//! or can be auto-approved based on the contributor's role.

use super::access::{OWNER, TRUSTED};

// ── Approval status ───────────────────────────────────────────────────────────

pub const PENDING:  &str = "pending";
pub const APPROVED: &str = "approved";
pub const REJECTED: &str = "rejected";

// ── Routing ───────────────────────────────────────────────────────────────────

/// Should this contribution auto-approve or go to the pending queue?
///
/// Owners and trusted members bypass the queue — their contributions
/// are immediately visible to all group members.
/// Contributors go to the pending queue for owner review.
pub fn route(role: &str) -> &'static str {
    match role {
        OWNER | TRUSTED => APPROVED,
        _               => PENDING,
    }
}

/// Should this node be visible to this member?
///
/// Approved nodes are visible to everyone.
/// Pending and rejected nodes are only visible to owners.
pub fn is_visible(node_status: &str, caller_role: &str) -> bool {
    match node_status {
        APPROVED           => true,
        PENDING | REJECTED => caller_role == OWNER,
        _                  => false,
    }
}

/// Is this contribution in a terminal state (cannot be changed)?
pub fn is_terminal(status: &str) -> bool {
    matches!(status, APPROVED | REJECTED)
}

// ── Duplicate detection threshold ─────────────────────────────────────────────

/// Cosine similarity score above which a contribution is flagged as a duplicate.
/// 0.92 = very similar — almost identical text with minor wording changes.
pub const DUPLICATE_THRESHOLD: f32 = 0.92;

pub fn is_duplicate(score: f32) -> bool {
    score >= DUPLICATE_THRESHOLD
}