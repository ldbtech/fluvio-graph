//! Database client for fluvio-collab.
//! All postgres read/writes go through this client.
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

use crate::clients::dbtypes::{DbUser, DbGroup, DbMember, DbInvite, DbQueueItem};
use crate::clients::parse_helpers::{parse_user, parse_group, parse_member, parse_invite, parse_queue_item};
// Client ---------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DatabaseClient {
    pub endpoint: String,
    pub client:   Client,
}

impl DatabaseClient {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self { endpoint: endpoint.into(), client: Client::new() }
    }
    
    // --- Users ------------------------------------------------------------
    pub async fn get_user(&self, id: &str) -> Result<Option<DbUser>, anyhow::Error> {
        let q = r#"query($id: String!) {
            getUser(id: $id) { id firebaseUid email displayName }
        }"#;
        let body = self.post(q, json!({ "id": id })).await?;
        Ok(parse_user(body["data"]["getUser"].clone()))
    }

    pub async fn get_user_by_firebase_uid(&self, uid: &str) -> anyhow::Result<Option<DbUser>> {
        let q = r#"query($uid: String!) {
            getUserByFirebaseUid(firebaseUid: $uid) { id firebaseUid email displayName }
        }"#;
        let body = self.post(q, json!({ "uid": uid })).await?;
        Ok(parse_user(body["data"]["getUserByFirebaseUid"].clone()))
    }

    pub async fn create_user(
        &self,
        firebase_uid: &str,
        email:        Option<&str>,
        display_name: Option<&str>,
    ) -> anyhow::Result<DbUser> {
        let q = r#"mutation($input: CreateUserInput!) {
            createUser(input: $input) { id firebaseUid email displayName }
        }"#;
        let body = self.post(q, json!({
            "input": { "firebaseUid": firebase_uid, "email": email, "displayName": display_name }
        })).await?;
        parse_user(body["data"]["createUser"].clone())
            .context("createUser returned null")
    }
 
    // ── Groups ────────────────────────────────────────────────────────────────
 
    pub async fn create_group(
        &self,
        name:        &str,
        description: Option<&str>,
        created_by:  &str,
    ) -> anyhow::Result<DbGroup> {
        let q = r#"mutation($input: CreateGroupInput!) {
            createGroup(input: $input) { id name description graphId createdBy }
        }"#;
        let body = self.post(q, json!({
            "input": { "name": name, "description": description, "createdBy": created_by }
        })).await?;
        parse_group(body["data"]["createGroup"].clone())
            .context("createGroup returned null")
    }
 
    pub async fn get_group(&self, id: &str) -> anyhow::Result<Option<DbGroup>> {
        let q = r#"query($id: String!) {
            getGroup(id: $id) { id name description graphId createdBy }
        }"#;
        let body = self.post(q, json!({ "id": id })).await?;
        Ok(parse_group(body["data"]["getGroup"].clone()))
    }
 
    pub async fn get_user_groups(&self, user_id: &str) -> anyhow::Result<Vec<DbGroup>> {
        let q = r#"query($userId: String!) {
            getUserGroups(userId: $userId) { id name description graphId createdBy }
        }"#;
        let body = self.post(q, json!({ "userId": user_id })).await?;
        Ok(body["data"]["getUserGroups"]
            .as_array().cloned().unwrap_or_default()
            .into_iter().filter_map(parse_group).collect())
    }
 
    // ── Members ───────────────────────────────────────────────────────────────
 
    pub async fn get_member(
        &self,
        group_id: &str,
        user_id:  &str,
    ) -> anyhow::Result<Option<DbMember>> {
        let q = r#"query($groupId: String!, $userId: String!) {
            getMember(groupId: $groupId, userId: $userId) {
                id groupId userId role invitedBy
            }
        }"#;
        let body = self.post(q, json!({ "groupId": group_id, "userId": user_id })).await?;
        Ok(parse_member(body["data"]["getMember"].clone()))
    }
 
    pub async fn get_group_members(&self, group_id: &str) -> anyhow::Result<Vec<DbMember>> {
        let q = r#"query($groupId: String!) {
            getGroupMembers(groupId: $groupId) { id groupId userId role invitedBy }
        }"#;
        let body = self.post(q, json!({ "groupId": group_id })).await?;
        Ok(body["data"]["getGroupMembers"]
            .as_array().cloned().unwrap_or_default()
            .into_iter().filter_map(parse_member).collect())
    }
 
    pub async fn add_member(
        &self,
        group_id:   &str,
        user_id:    &str,
        role:       &str,
        invited_by: Option<&str>,
    ) -> anyhow::Result<DbMember> {
        let q = r#"mutation($input: AddMemberInput!) {
            addMember(input: $input) { id groupId userId role invitedBy }
        }"#;
        let body = self.post(q, json!({
            "input": {
                "groupId": group_id, "userId": user_id,
                "role": role, "invitedBy": invited_by
            }
        })).await?;
        parse_member(body["data"]["addMember"].clone())
            .context("addMember returned null")
    }
 
    pub async fn update_member_role(
        &self,
        group_id: &str,
        user_id:  &str,
        new_role: &str,
    ) -> anyhow::Result<DbMember> {
        let q = r#"mutation($groupId: String!, $userId: String!, $newRole: String!) {
            updateMemberRole(groupId: $groupId, userId: $userId, newRole: $newRole) {
                id groupId userId role invitedBy
            }
        }"#;
        let body = self.post(q, json!({
            "groupId": group_id, "userId": user_id, "newRole": new_role
        })).await?;
        parse_member(body["data"]["updateMemberRole"].clone())
            .context("updateMemberRole returned null")
    }
 
    // ── Invites ───────────────────────────────────────────────────────────────
 
    pub async fn create_invite(
        &self,
        group_id:   &str,
        invited_by: &str,
        role:       &str,
        email:      Option<&str>,
    ) -> anyhow::Result<DbInvite> {
        let q = r#"mutation($input: CreateInviteInput!) {
            createInvite(input: $input) { id groupId token role email expiresAt }
        }"#;
        let body = self.post(q, json!({
            "input": {
                "groupId": group_id, "invitedBy": invited_by,
                "role": role, "email": email, "expiresInHours": 72
            }
        })).await?;
        parse_invite(body["data"]["createInvite"].clone())
            .context("createInvite returned null")
    }
 
    pub async fn get_invite_by_token(&self, token: &str) -> anyhow::Result<Option<DbInvite>> {
        let q = r#"query($token: String!) {
            getInviteByToken(token: $token) { id groupId token role email expiresAt }
        }"#;
        let body = self.post(q, json!({ "token": token })).await?;
        Ok(parse_invite(body["data"]["getInviteByToken"].clone()))
    }
 
    pub async fn accept_invite(
        &self,
        token:       &str,
        accepted_by: &str,
    ) -> anyhow::Result<DbInvite> {
        let q = r#"mutation($token: String!, $acceptedBy: String!) {
            acceptInvite(token: $token, acceptedBy: $acceptedBy) {
                id groupId token role email expiresAt
            }
        }"#;
        let body = self.post(q, json!({ "token": token, "acceptedBy": accepted_by })).await?;
        parse_invite(body["data"]["acceptInvite"].clone())
            .context("acceptInvite returned null")
    }
 
    // ── Approval queue ────────────────────────────────────────────────────────
 
    pub async fn submit_to_queue(
        &self,
        group_id:        &str,
        contributed_by:  &str,
        kind:            &str,
        surreal_node_id: &str,
    ) -> anyhow::Result<DbQueueItem> {
        let q = r#"mutation($input: SubmitToQueueInput!) {
            submitToQueue(input: $input) {
                id groupId contributedBy kind surrealNodeId status reviewNote
            }
        }"#;
        let body = self.post(q, json!({
            "input": {
                "groupId": group_id, "contributedBy": contributed_by,
                "kind": kind, "surrealNodeId": surreal_node_id
            }
        })).await?;
        parse_queue_item(body["data"]["submitToQueue"].clone())
            .context("submitToQueue returned null")
    }
 
    pub async fn get_pending_queue(&self, group_id: &str) -> anyhow::Result<Vec<DbQueueItem>> {
        let q = r#"query($groupId: String!) {
            getPendingQueue(groupId: $groupId) {
                id groupId contributedBy kind surrealNodeId status reviewNote
            }
        }"#;
        let body = self.post(q, json!({ "groupId": group_id })).await?;
        Ok(body["data"]["getPendingQueue"]
            .as_array().cloned().unwrap_or_default()
            .into_iter().filter_map(parse_queue_item).collect())
    }
 
    pub async fn update_queue_status(
        &self,
        id:          &str,
        status:      &str,
        reviewed_by: &str,
        note:        Option<&str>,
    ) -> anyhow::Result<DbQueueItem> {
        let q = r#"mutation($input: UpdateQueueStatusInput!) {
            updateQueueStatus(input: $input) {
                id groupId contributedBy kind surrealNodeId status reviewNote
            }
        }"#;
        let body = self.post(q, json!({
            "input": {
                "id": id, "status": status,
                "reviewedBy": reviewed_by, "reviewNote": note
            }
        })).await?;
        parse_queue_item(body["data"]["updateQueueStatus"].clone())
            .context("updateQueueStatus returned null")
    }
 
    // ── Internal ──────────────────────────────────────────────────────────────
 
    async fn post(&self, query: &str, variables: Value) -> anyhow::Result<Value> {
        let resp = self.client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .context("failed to reach fluvio-database")?;
 
        let body: Value = resp.json().await
            .context("failed to parse fluvio-database response")?;
 
        if let Some(errors) = body.get("errors") {
            anyhow::bail!("fluvio-database error: {errors}");
        }
 
        Ok(body)
    }
}

// -- Parse Helpers -------------------------------------------------------