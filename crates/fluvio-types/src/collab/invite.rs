//! Invite types for bringing new members into a group.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collab::group::{GroupId, Role};

// ── InviteToken ───────────────────────────────────────────────────────────────

/// A signed, single-use token embedded in an invite link.
/// Format: random UUID stored in Postgres `invites.token`.
/// The HTTP layer verifies it before creating a `group_members` record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteToken(pub String);

impl InviteToken {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for InviteToken {
    fn default() -> Self { Self::new() }
}

// ── Invite ────────────────────────────────────────────────────────────────────

/// A pending or consumed group invitation.
/// Maps to Postgres `invites` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id:          Uuid,
    pub group_id:    GroupId,
    /// Who sent the invite (must be the group owner).
    pub invited_by:  Uuid,
    pub token:       InviteToken,
    /// Optional target email — if set, only that email can accept.
    pub email:       Option<String>,
    /// Role the invitee will receive on acceptance.
    pub role:        Role,
    pub expires_at:  DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

impl Invite {
    /// Create a new invite that expires in `hours_valid` hours.
    pub fn new(
        group_id:    GroupId,
        invited_by:  Uuid,
        role:        Role,
        email:       Option<String>,
        hours_valid: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id:          Uuid::new_v4(),
            group_id,
            invited_by,
            token:       InviteToken::new(),
            email,
            role,
            expires_at:  now + chrono::Duration::hours(hours_valid),
            accepted_at: None,
            created_at:  now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_accepted(&self) -> bool {
        self.accepted_at.is_some()
    }
}