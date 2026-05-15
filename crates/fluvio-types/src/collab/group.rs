//! Collaborative graph group types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── GroupId ───────────────────────────────────────────────────────────────────

/// Stable identifier for a collaborative graph group.
/// Stored in Postgres `groups.id` and used as the `group_id` discriminator
/// in every SurrealDB collab node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub Uuid);

impl GroupId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for GroupId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── Role ──────────────────────────────────────────────────────────────────────

/// A member's role within a collaborative graph group.
///
/// - `Owner` — created the group; can invite, approve/reject contributions,
///   and delete the group. Only one owner per group.
/// - `Contributor` — can add nodes, tools, agents, PDFs; contributions require
///   owner approval before becoming live in the graph.
/// - `Viewer` — read-only access to the graph; can query and run approved tools
///   but cannot mutate anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Contributor,
    Viewer,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Role::Owner       => "owner",
            Role::Contributor => "contributor",
            Role::Viewer      => "viewer",
        };
        write!(f, "{s}")
    }
}

// ── Group ─────────────────────────────────────────────────────────────────────

/// A collaborative graph group.
///
/// One group = one shared knowledge graph in SurrealDB-collab.
/// Membership and access control live in Postgres; graph data lives in Surreal.
///
/// The `graph_id` field is the SurrealDB namespace key — it mirrors `id`
/// so the storage layer can use `group_id` as the Surreal discriminator
/// without an extra join.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id:          GroupId,
    /// UUID of the owning user (Postgres `users.id`).
    pub owner_id:    Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

impl Group {
    pub fn new(owner_id: Uuid, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id:          GroupId::new(),
            owner_id,
            name:        name.into(),
            description: None,
            created_at:  now,
            updated_at:  now,
        }
    }
}