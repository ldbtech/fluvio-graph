//! GraphQL types for fluvio-database subgraph.

use async_graphql::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::queries::{users::User, groups::Group, members::Member,
                invites::Invite, queue::QueueItem};

// ── User ─────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlUser {
    pub id:           String,
    pub firebase_uid: String,
    pub email:        Option<String>,
    pub display_name: Option<String>,
    pub avatar_url:   Option<String>,
    pub created_at:   String,
}

impl From<User> for GqlUser {
    fn from(u: User) -> Self {
        Self {
            id:           u.id.to_string(),
            firebase_uid: u.firebase_uid,
            email:        u.email,
            display_name: u.display_name,
            avatar_url:   u.avatar_url,
            created_at:   u.created_at.to_rfc3339(),
        }
    }
}

// ── Group ─────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlGroup {
    pub id:          String,
    pub name:        String,
    pub description: Option<String>,
    pub graph_id:    String,
    pub created_by:  String,
    pub created_at:  String,
}

impl From<Group> for GqlGroup {
    fn from(g: Group) -> Self {
        Self {
            id:          g.id.to_string(),
            name:        g.name,
            description: g.description,
            graph_id:    g.graph_id.to_string(),
            created_by:  g.created_by.to_string(),
            created_at:  g.created_at.to_rfc3339(),
        }
    }
}

// ── Member ────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlMember {
    pub id:         String,
    pub group_id:   String,
    pub user_id:    String,
    pub role:       String,
    pub invited_by: Option<String>,
    pub joined_at:  String,
}

impl From<Member> for GqlMember {
    fn from(m: Member) -> Self {
        Self {
            id:         m.id.to_string(),
            group_id:   m.group_id.to_string(),
            user_id:    m.user_id.to_string(),
            role:       m.role,
            invited_by: m.invited_by.map(|u| u.to_string()),
            joined_at:  m.joined_at.to_rfc3339(),
        }
    }
}

// ── Invite ────────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlInvite {
    pub id:          String,
    pub group_id:    String,
    pub invited_by:  String,
    pub token:       String,
    pub role:        String,
    pub email:       Option<String>,
    pub expires_at:  String,
    pub accepted_at: Option<String>,
    pub created_at:  String,
}

impl From<Invite> for GqlInvite {
    fn from(i: Invite) -> Self {
        Self {
            id:          i.id.to_string(),
            group_id:    i.group_id.to_string(),
            invited_by:  i.invited_by.to_string(),
            token:       i.token,
            role:        i.role,
            email:       i.email,
            expires_at:  i.expires_at.to_rfc3339(),
            accepted_at: i.accepted_at.map(|t| t.to_rfc3339()),
            created_at:  i.created_at.to_rfc3339(),
        }
    }
}

// ── QueueItem ─────────────────────────────────────────────────────────────────

#[derive(SimpleObject, Clone)]
pub struct GqlQueueItem {
    pub id:              String,
    pub group_id:        String,
    pub contributed_by:  String,
    pub kind:            String,
    pub surreal_node_id: String,
    pub status:          String,
    pub reviewed_by:     Option<String>,
    pub review_note:     Option<String>,
    pub created_at:      String,
    pub reviewed_at:     Option<String>,
}

impl From<QueueItem> for GqlQueueItem {
    fn from(q: QueueItem) -> Self {
        Self {
            id:              q.id.to_string(),
            group_id:        q.group_id.to_string(),
            contributed_by:  q.contributed_by.to_string(),
            kind:            q.kind,
            surreal_node_id: q.surreal_node_id,
            status:          q.status,
            reviewed_by:     q.reviewed_by.map(|u| u.to_string()),
            review_note:     q.review_note,
            created_at:      q.created_at.to_rfc3339(),
            reviewed_at:     q.reviewed_at.map(|t| t.to_rfc3339()),
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct CreateUserInput {
    pub firebase_uid: String,
    pub email:        Option<String>,
    pub display_name: Option<String>,
    pub avatar_url:   Option<String>,
}

#[derive(InputObject)]
pub struct CreateGroupInput {
    pub name:        String,
    pub description: Option<String>,
    pub created_by:  String,
}

#[derive(InputObject)]
pub struct AddMemberInput {
    pub group_id:   String,
    pub user_id:    String,
    pub role:       String,
    pub invited_by: Option<String>,
}

#[derive(InputObject)]
pub struct CreateInviteInput {
    pub group_id:   String,
    pub invited_by: String,
    pub role:       String,
    pub email:      Option<String>,
    /// Hours until expiry — default 72
    pub expires_in_hours: Option<i32>,
}

#[derive(InputObject)]
pub struct SubmitToQueueInput {
    pub group_id:        String,
    pub contributed_by:  String,
    pub kind:            String,
    pub surreal_node_id: String,
}

#[derive(InputObject)]
pub struct UpdateQueueStatusInput {
    pub id:          String,
    pub status:      String,
    pub reviewed_by: String,
    pub review_note: Option<String>,
}