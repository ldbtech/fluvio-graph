//! Database client for fluvio-collab.
//! All postgres read/writes go through this client.
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;
use anyhow::Context;

use crate::clients::dbtypes::{DbUser, DbGroup, DbMember, DbInvite, DbQueueItem, DbCompany, DbCompanyInvite, DbTeam, DbTeamMember, DbTeamWorkflow};
use crate::clients::parse_helpers::{parse_user, parse_group, parse_member, parse_invite, parse_queue_item, parse_company, parse_company_invite, parse_team, parse_team_member, parse_team_workflow};
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
            getUser(id: $id) { id firebaseUid email displayName companyEmail companyId }
        }"#;
        let body = self.post(q, json!({ "id": id })).await?;
        Ok(parse_user(body["data"]["getUser"].clone()))
    }

    pub async fn get_user_by_firebase_uid(&self, uid: &str) -> anyhow::Result<Option<DbUser>> {
        let q = r#"query($uid: String!) {
            getUserByFirebaseUid(firebaseUid: $uid) { id firebaseUid email displayName companyEmail companyId }
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
            createUser(input: $input) { id firebaseUid email displayName companyEmail companyId }
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

    // ── Company & Teams ──────────────────────────────────────────────────────────

    pub async fn update_user_company_email(&self, user_id: &str, email: &str) -> anyhow::Result<DbUser> {
        let q = r#"mutation($userId: String!, $email: String!) {
            updateUserCompanyEmail(userId: $userId, email: $email) { id firebaseUid email displayName companyEmail companyId }
        }"#;
        let body = self.post(q, json!({ "userId": user_id, "email": email })).await?;
        parse_user(body["data"]["updateUserCompanyEmail"].clone())
            .context("updateUserCompanyEmail returned null")
    }

    pub async fn update_user_company(&self, user_id: &str, company_id: &str) -> anyhow::Result<DbUser> {
        let q = r#"mutation($userId: String!, $companyId: String!) {
            updateUserCompany(userId: $userId, companyId: $companyId) { id firebaseUid email displayName companyEmail companyId }
        }"#;
        let body = self.post(q, json!({ "userId": user_id, "companyId": company_id })).await?;
        parse_user(body["data"]["updateUserCompany"].clone())
            .context("updateUserCompany returned null")
    }

    pub async fn create_company(
        &self,
        name:         &str,
        website:      &str,
        linkedin_url: &str,
        twitter_url:  Option<&str>,
        github_url:   Option<&str>,
        created_by:   &str,
    ) -> anyhow::Result<DbCompany> {
        let q = r#"mutation($input: CreateCompanyInput!) {
            createCompany(input: $input) { id name website linkedinUrl twitterUrl githubUrl createdBy }
        }"#;
        let body = self.post(q, json!({
            "input": {
                "name": name,
                "website": website,
                "linkedinUrl": linkedin_url,
                "twitterUrl": twitter_url,
                "githubUrl": github_url,
                "createdBy": created_by
            }
        })).await?;
        parse_company(body["data"]["createCompany"].clone())
            .context("createCompany returned null")
    }

    pub async fn get_company(&self, id: &str) -> anyhow::Result<Option<DbCompany>> {
        let q = r#"query($id: String!) {
            getCompany(id: $id) { id name website linkedinUrl twitterUrl githubUrl createdBy }
        }"#;
        let body = self.post(q, json!({ "id": id })).await?;
        Ok(parse_company(body["data"]["getCompany"].clone()))
    }

    pub async fn get_pending_company_invites(&self, email: &str) -> anyhow::Result<Vec<DbCompanyInvite>> {
        let q = r#"query($email: String!) {
            getPendingCompanyInvites(email: $email) { id companyId invitedBy email token role expiresAt acceptedAt }
        }"#;
        let body = self.post(q, json!({ "email": email })).await?;
        Ok(body["data"]["getPendingCompanyInvites"]
            .as_array().cloned().unwrap_or_default()
            .into_iter().filter_map(parse_company_invite).collect())
    }

    pub async fn create_company_invite(
        &self,
        company_id: &str,
        invited_by: &str,
        email:      &str,
        token:      &str,
        role:       &str,
        expires_at: &str,
    ) -> anyhow::Result<DbCompanyInvite> {
        let q = r#"mutation($input: CreateCompanyInviteInput!) {
            createCompanyInvite(input: $input) { id companyId invitedBy email token role expiresAt acceptedAt }
        }"#;
        let body = self.post(q, json!({
            "input": {
                "companyId": company_id,
                "invitedBy": invited_by,
                "email": email,
                "token": token,
                "role": role,
                "expiresAt": expires_at
            }
        })).await?;
        parse_company_invite(body["data"]["createCompanyInvite"].clone())
            .context("createCompanyInvite returned null")
    }

    pub async fn accept_company_invite(&self, invite_id: &str) -> anyhow::Result<DbCompanyInvite> {
        let q = r#"mutation($inviteId: String!) {
            acceptCompanyInvite(inviteId: $inviteId) { id companyId invitedBy email token role expiresAt acceptedAt }
        }"#;
        let body = self.post(q, json!({ "inviteId": invite_id })).await?;
        parse_company_invite(body["data"]["acceptCompanyInvite"].clone())
            .context("acceptCompanyInvite returned null")
    }

    pub async fn create_team(
        &self,
        company_id:  &str,
        name:        &str,
        description: Option<&str>,
    ) -> anyhow::Result<DbTeam> {
        let q = r#"mutation($input: CreateTeamInput!) {
            createTeam(input: $input) { id companyId name description }
        }"#;
        let body = self.post(q, json!({
            "input": { "companyId": company_id, "name": name, "description": description }
        })).await?;
        parse_team(body["data"]["createTeam"].clone())
            .context("createTeam returned null")
    }

    pub async fn get_team(&self, id: &str) -> anyhow::Result<Option<DbTeam>> {
        let q = r#"query($id: String!) {
            getTeam(id: $id) { id companyId name description }
        }"#;
        let body = self.post(q, json!({ "id": id })).await?;
        Ok(parse_team(body["data"]["getTeam"].clone()))
    }

    pub async fn get_company_teams(&self, company_id: &str) -> anyhow::Result<Vec<DbTeam>> {
        let q = r#"query($companyId: String!) {
            getCompanyTeams(companyId: $companyId) { id companyId name description }
        }"#;
        let body = self.post(q, json!({ "companyId": company_id })).await?;
        Ok(body["data"]["getCompanyTeams"]
            .as_array().cloned().unwrap_or_default()
            .into_iter().filter_map(parse_team).collect())
    }

    pub async fn add_team_member(
        &self,
        team_id: &str,
        user_id: &str,
        role:    &str,
    ) -> anyhow::Result<DbTeamMember> {
        let q = r#"mutation($input: AddTeamMemberInput!) {
            addTeamMember(input: $input) { id teamId userId role joinedAt }
        }"#;
        let body = self.post(q, json!({
            "input": { "teamId": team_id, "userId": user_id, "role": role }
        })).await?;
        parse_team_member(body["data"]["addTeamMember"].clone())
            .context("addTeamMember returned null")
    }

    pub async fn create_team_workflow(
        &self,
        team_id:     &str,
        name:        &str,
        description: Option<&str>,
        steps:       &str,
        created_by:  &str,
    ) -> anyhow::Result<DbTeamWorkflow> {
        let q = r#"mutation($input: CreateTeamWorkflowInput!) {
            createTeamWorkflow(input: $input) { id teamId name description steps createdBy }
        }"#;
        let body = self.post(q, json!({
            "input": { "teamId": team_id, "name": name, "description": description, "steps": steps, "createdBy": created_by }
        })).await?;
        parse_team_workflow(body["data"]["createTeamWorkflow"].clone())
            .context("createTeamWorkflow returned null")
    }

    pub async fn get_team_workflows(&self, team_id: &str) -> anyhow::Result<Vec<DbTeamWorkflow>> {
        let q = r#"query($teamId: String!) {
            getTeamWorkflows(teamId: $teamId) { id teamId name description steps createdBy }
        }"#;
        let body = self.post(q, json!({ "teamId": team_id })).await?;
        Ok(body["data"]["getTeamWorkflows"]
            .as_array().cloned().unwrap_or_default()
            .into_iter().filter_map(parse_team_workflow).collect())
    }
}

// -- Parse Helpers -------------------------------------------------------