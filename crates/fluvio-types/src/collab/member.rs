//! Group membership types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collab::group::{GroupId, Role};

// ── MemberStatus ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    /// Invite accepted, membership active.
    Active,
    /// Invite sent but not yet accepted.
    Pending,
    /// Owner has revoked access.
    Revoked,
}

// ── Member ────────────────────────────────────────────────────────────────────

/// A user's membership record within a group.
/// Maps to Postgres `group_members` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id:          Uuid,
    pub group_id:    GroupId,
    /// Postgres `users.id` of this member.
    pub user_id:     Uuid,
    pub role:        Role,
    pub status:      MemberStatus,
    /// Who sent the invite (Postgres `users.id`).
    pub invited_by:  Uuid,
    pub joined_at:   Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}