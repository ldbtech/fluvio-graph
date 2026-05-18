use serde_json::Value;
use crate::clients::dbtypes::{DbUser, DbGroup, DbMember, DbInvite, DbQueueItem};

// ── Parse helpers ──────────────────────────────────────────────────────────────

pub fn parse_user(v: Value) -> Option<DbUser> {
    Some(DbUser {
        id:           v["id"].as_str()?.to_string(),
        firebase_uid: v["firebaseUid"].as_str()?.to_string(),
        email:        v["email"].as_str().map(String::from),
        display_name: v["displayName"].as_str().map(String::from),
    })
}
 
pub fn parse_group(v: Value) -> Option<DbGroup> {
    Some(DbGroup {
        id:          v["id"].as_str()?.to_string(),
        name:        v["name"].as_str()?.to_string(),
        description: v["description"].as_str().map(String::from),
        graph_id:    v["graphId"].as_str()?.to_string(),
        created_by:  v["createdBy"].as_str()?.to_string(),
    })
}
 
pub fn parse_member(v: Value) -> Option<DbMember> {
    Some(DbMember {
        id:         v["id"].as_str()?.to_string(),
        group_id:   v["groupId"].as_str()?.to_string(),
        user_id:    v["userId"].as_str()?.to_string(),
        role:       v["role"].as_str()?.to_string(),
        invited_by: v["invitedBy"].as_str().map(String::from),
    })
}
 
pub fn parse_invite(v: Value) -> Option<DbInvite> {
    Some(DbInvite {
        id:         v["id"].as_str()?.to_string(),
        group_id:   v["groupId"].as_str()?.to_string(),
        token:      v["token"].as_str()?.to_string(),
        role:       v["role"].as_str()?.to_string(),
        email:      v["email"].as_str().map(String::from),
        expires_at: v["expiresAt"].as_str()?.to_string(),
    })
}
 
pub fn parse_queue_item(v: Value) -> Option<DbQueueItem> {
    Some(DbQueueItem {
        id:              v["id"].as_str()?.to_string(),
        group_id:        v["groupId"].as_str()?.to_string(),
        contributed_by:  v["contributedBy"].as_str()?.to_string(),
        kind:            v["kind"].as_str()?.to_string(),
        surreal_node_id: v["surrealNodeId"].as_str()?.to_string(),
        status:          v["status"].as_str()?.to_string(),
        review_note:     v["reviewNote"].as_str().map(String::from),
    })
}
 