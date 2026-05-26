//! Mutation resolvers for fluvio-database.

use async_graphql::*;
use uuid::Uuid;
use chrono::Utc;
use chrono::DateTime;

use crate::server::AppState;
use crate::db::{users, groups, members, invites, queue, companies, teams};
use crate::graphql::types::*;

use crate::graphql::connectors_type;
use crate::graphql::connectors_mutation;

pub struct MutationRoot;

#[Object(name = "Mutation")]
impl MutationRoot {

    async fn create_user(
        &self,
        ctx:   &Context<'_>,
        input: CreateUserInput,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        Ok(users::create_user(
            &state.pool,
            &input.firebase_uid,
            input.email.as_deref(),
            input.display_name.as_deref(),
            input.avatar_url.as_deref(),
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlUser::from)?)
    }

    async fn create_group(
        &self,
        ctx:   &Context<'_>,
        input: CreateGroupInput,
    ) -> Result<GqlGroup> {
        let state      = ctx.data::<AppState>()?;
        let created_by = parse_uuid(&input.created_by)?;
        Ok(groups::create_group(
            &state.pool,
            &input.name,
            input.description.as_deref(),
            created_by,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlGroup::from)?)
    }

    async fn add_member(
        &self,
        ctx:   &Context<'_>,
        input: AddMemberInput,
    ) -> Result<GqlMember> {
        let state      = ctx.data::<AppState>()?;
        let group_id   = parse_uuid(&input.group_id)?;
        let user_id    = parse_uuid(&input.user_id)?;
        let invited_by = input.invited_by
            .as_deref()
            .map(parse_uuid)
            .transpose()?;

        Ok(members::add_member(
            &state.pool,
            group_id,
            user_id,
            &input.role,
            invited_by,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlMember::from)?)
    }

    async fn update_member_role(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        user_id:  String,
        new_role: String,
    ) -> Result<GqlMember> {
        let state    = ctx.data::<AppState>()?;
        let group_id = parse_uuid(&group_id)?;
        let user_id  = parse_uuid(&user_id)?;
        Ok(members::update_member_role(
            &state.pool, group_id, user_id, &new_role,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlMember::from)?)
    }

    async fn remove_member(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        user_id:  String,
    ) -> Result<bool> {
        let state    = ctx.data::<AppState>()?;
        let group_id = parse_uuid(&group_id)?;
        let user_id  = parse_uuid(&user_id)?;

        // Ensure group still has at least one owner after removal
        let owner_count = members::get_group_owner_count(&state.pool, group_id).await
            .map_err(|e| Error::new(e.to_string()))?;

        let member = members::get_member(&state.pool, group_id, user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("member not found"))?;

        if member.role == "owner" && owner_count <= 1 {
            return Err(Error::new(
                "cannot remove the last owner — promote another member first"
            ));
        }

        Ok(members::remove_member(&state.pool, group_id, user_id).await
            .map_err(|e| Error::new(e.to_string()))?)
    }

    async fn create_invite(
        &self,
        ctx:   &Context<'_>,
        input: CreateInviteInput,
    ) -> Result<GqlInvite> {
        let state      = ctx.data::<AppState>()?;
        let group_id   = parse_uuid(&input.group_id)?;
        let invited_by = parse_uuid(&input.invited_by)?;
        let hours      = input.expires_in_hours.unwrap_or(72) as i64;
        let expires_at = Utc::now() + chrono::Duration::hours(hours);

        Ok(invites::create_invite(
            &state.pool,
            group_id,
            invited_by,
            &input.role,
            input.email.as_deref(),
            expires_at,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlInvite::from)?)
    }

    async fn accept_invite(
        &self,
        ctx:         &Context<'_>,
        token:       String,
        accepted_by: String,
    ) -> Result<GqlInvite> {
        let state       = ctx.data::<AppState>()?;
        let accepted_by = parse_uuid(&accepted_by)?;
        Ok(invites::accept_invite(&state.pool, &token, accepted_by).await
            .map_err(|e| Error::new(e.to_string()))
            .map(GqlInvite::from)?)
    }

    async fn submit_to_queue(
        &self,
        ctx:   &Context<'_>,
        input: SubmitToQueueInput,
    ) -> Result<GqlQueueItem> {
        let state          = ctx.data::<AppState>()?;
        let group_id       = parse_uuid(&input.group_id)?;
        let contributed_by = parse_uuid(&input.contributed_by)?;
        Ok(queue::submit_to_queue(
            &state.pool,
            group_id,
            contributed_by,
            &input.kind,
            &input.surreal_node_id,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlQueueItem::from)?)
    }

    async fn update_queue_status(
        &self,
        ctx:   &Context<'_>,
        input: UpdateQueueStatusInput,
    ) -> Result<GqlQueueItem> {
        let state       = ctx.data::<AppState>()?;
        let id          = parse_uuid(&input.id)?;
        let reviewed_by = parse_uuid(&input.reviewed_by)?;
        Ok(queue::update_queue_status(
            &state.pool,
            id,
            &input.status,
            reviewed_by,
            input.review_note.as_deref(),
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlQueueItem::from)?)
    }

    async fn create_connector(
        &self, ctx: &Context<'_>, input: connectors_type::CreateConnectorInput,
    ) -> Result<connectors_type::GqlConnector> {
        connectors_mutation::create_connector(ctx, input).await
    }
    
    async fn upsert_resource(
        &self, ctx: &Context<'_>, input: connectors_type::UpsertResourceInput,
    ) -> Result<connectors_type::GqlConnectorResource> {
        connectors_mutation::upsert_resource(ctx, input).await
    }
    
    async fn select_resources(
        &self, ctx: &Context<'_>, input: connectors_type::SelectResourcesInput,
    ) -> Result<Vec<connectors_type::GqlConnectorResource>> {
        connectors_mutation::select_resources(ctx, input).await
    }
    
    async fn update_connector_status(
        &self, ctx: &Context<'_>, connector_id: String,
        status: String, error: Option<String>,
    ) -> Result<connectors_type::GqlConnector> {
        connectors_mutation::update_connector_status(ctx, connector_id, status, error).await
    }
    
    async fn mark_synced(
        &self, ctx: &Context<'_>, connector_id: String,
    ) -> Result<connectors_type::GqlConnector> {
        connectors_mutation::mark_synced(ctx, connector_id).await
    }
    
    async fn update_resource_sync_stats(
        &self, ctx: &Context<'_>, connector_id: String,
        external_id: String, nodes_added: i32,
    ) -> Result<bool> {
        connectors_mutation::update_resource_sync_stats(ctx, connector_id, external_id, nodes_added).await
    }
    
    async fn disconnect_connector(
        &self, ctx: &Context<'_>, connector_id: String,
    ) -> Result<bool> {
        connectors_mutation::disconnect_connector(ctx, connector_id).await
    }

    async fn create_workspace(
        &self,
        ctx:   &Context<'_>,
        input: CreateWorkspaceInput,
    ) -> Result<GqlWorkspace> {
        let state    = ctx.data::<AppState>()?;
        let owner_id = parse_uuid(&input.owner_id)?;
        Ok(crate::db::workspaces::create_workspace(
            &state.pool,
            owner_id,
            &input.name,
            input.is_public,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlWorkspace::from)?)
    }

    async fn update_workspace(
        &self,
        ctx:   &Context<'_>,
        input: UpdateWorkspaceInput,
    ) -> Result<GqlWorkspace> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&input.id)?;
        Ok(crate::db::workspaces::update_workspace(
            &state.pool,
            id,
            input.name.as_deref(),
            input.is_public,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlWorkspace::from)?)
    }

    async fn delete_workspace(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        crate::db::workspaces::delete_workspace(&state.pool, id).await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(true)
    }

    async fn share_workspace(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
        email:        String,
    ) -> Result<GqlWorkspaceShare> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        
        // Find user by email
        let user = users::get_user_by_email(&state.pool, &email).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new(format!("User with email {} not found", email)))?;
            
        let share = crate::db::workspaces::share_workspace(&state.pool, workspace_id, user.id).await
            .map_err(|e| Error::new(e.to_string()))?;
            
        Ok(GqlWorkspaceShare {
            id:           share.id.to_string(),
            workspace_id: share.workspace_id.to_string(),
            user_id:      share.user_id.to_string(),
            shared_at:    share.shared_at.to_rfc3339(),
            email:        user.email,
            display_name: user.display_name,
        })
    }

    async fn unshare_workspace(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
        user_id:      String,
    ) -> Result<bool> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        let user_id      = parse_uuid(&user_id)?;
        crate::db::workspaces::unshare_workspace(&state.pool, workspace_id, user_id).await
            .map_err(|e| Error::new(e.to_string()))?;
        Ok(true)
    }

    // ── Company & Teams Mutations ──────────────────────────────────────────────────

    async fn create_company(
        &self,
        ctx:   &Context<'_>,
        input: CreateCompanyInput,
    ) -> Result<GqlCompany> {
        let state      = ctx.data::<AppState>()?;
        let created_by = parse_uuid(&input.created_by)?;
        Ok(companies::create_company(
            &state.pool,
            &input.name,
            &input.website,
            &input.linkedin_url,
            input.twitter_url.as_deref(),
            input.github_url.as_deref(),
            created_by,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlCompany::from)?)
    }

    async fn update_user_company_email(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
        email:   String,
    ) -> Result<GqlUser> {
        let state   = ctx.data::<AppState>()?;
        let user_id = parse_uuid(&user_id)?;
        Ok(users::update_company_email(
            &state.pool,
            user_id,
            Some(&email),
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlUser::from)?)
    }

    async fn update_user_company(
        &self,
        ctx:        &Context<'_>,
        user_id:    String,
        company_id: String,
    ) -> Result<GqlUser> {
        let state      = ctx.data::<AppState>()?;
        let user_id    = parse_uuid(&user_id)?;
        let company_id = parse_uuid(&company_id)?;
        Ok(users::update_company_id(
            &state.pool,
            user_id,
            Some(company_id),
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlUser::from)?)
    }

    async fn create_company_invite(
        &self,
        ctx:   &Context<'_>,
        input: CreateCompanyInviteInput,
    ) -> Result<GqlCompanyInvite> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&input.company_id)?;
        let invited_by = parse_uuid(&input.invited_by)?;
        let expires_at = chrono::DateTime::parse_from_rfc3339(&input.expires_at)
            .map_err(|e: chrono::ParseError| Error::new(e.to_string()))?
            .with_timezone(&Utc);

        Ok(companies::create_company_invite(
            &state.pool,
            company_id,
            invited_by,
            &input.email,
            &input.token,
            &input.role,
            expires_at,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlCompanyInvite::from)?)
    }

    async fn accept_company_invite(
        &self,
        ctx:       &Context<'_>,
        invite_id: String,
    ) -> Result<GqlCompanyInvite> {
        let state     = ctx.data::<AppState>()?;
        let invite_id = parse_uuid(&invite_id)?;
        Ok(companies::accept_company_invite(
            &state.pool,
            invite_id,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlCompanyInvite::from)?)
    }

    async fn create_team(
        &self,
        ctx:   &Context<'_>,
        input: CreateTeamInput,
    ) -> Result<GqlTeam> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&input.company_id)?;
        Ok(teams::create_team(
            &state.pool,
            company_id,
            &input.name,
            input.description.as_deref(),
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlTeam::from)?)
    }

    async fn add_team_member(
        &self,
        ctx:   &Context<'_>,
        input: AddTeamMemberInput,
    ) -> Result<GqlTeamMember> {
        let state   = ctx.data::<AppState>()?;
        let team_id = parse_uuid(&input.team_id)?;
        let user_id = parse_uuid(&input.user_id)?;
        Ok(teams::add_team_member(
            &state.pool,
            team_id,
            user_id,
            &input.role,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlTeamMember::from)?)
    }

    async fn create_team_workflow(
        &self,
        ctx:   &Context<'_>,
        input: CreateTeamWorkflowInput,
    ) -> Result<GqlTeamWorkflow> {
        let state      = ctx.data::<AppState>()?;
        let team_id    = parse_uuid(&input.team_id)?;
        let created_by = parse_uuid(&input.created_by)?;
        let steps_json: serde_json::Value = serde_json::from_str(&input.steps)
            .map_err(|e| Error::new(format!("Invalid steps JSON: {e}")))?;

        Ok(teams::create_team_workflow(
            &state.pool,
            team_id,
            &input.name,
            input.description.as_deref(),
            steps_json,
            created_by,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlTeamWorkflow::from)?)
    }

    async fn add_planner_chat_message(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
        sender:       String,
        content:      String,
    ) -> Result<GqlPlannerChatMessage> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        let user_id      = extract_user_id(ctx)?;
        
        crate::db::workspaces::verify_workspace_access(&state.pool, workspace_id, user_id).await
            .map_err(|e| Error::new(format!("Access Denied: {}", e)))?;

        let message = sqlx::query_as::<_, GqlPlannerChatMessage>(
            "INSERT INTO planner_chat_messages (workspace_id, sender, content)
             VALUES ($1, $2, $3)
             RETURNING id::text, workspace_id::text, sender, content, created_at::text"
        )
        .bind(workspace_id)
        .bind(sender)
        .bind(content)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(message)
    }

    async fn clear_planner_chat_history(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
    ) -> Result<bool> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        let user_id      = extract_user_id(ctx)?;
        
        crate::db::workspaces::verify_workspace_access(&state.pool, workspace_id, user_id).await
            .map_err(|e| Error::new(format!("Access Denied: {}", e)))?;

        sqlx::query("DELETE FROM planner_chat_messages WHERE workspace_id = $1")
            .bind(workspace_id)
            .execute(&state.pool)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(true)
    }

    async fn resolve_action_authorization(
        &self,
        ctx:    &Context<'_>,
        id:     String,
        status: String,
        notes:  Option<String>,
    ) -> Result<GqlActionAuthorization> {
        let state                 = ctx.data::<AppState>()?;
        let id                    = parse_uuid(&id)?;
        let authorized_by_user_id = extract_user_id(ctx).ok();
        
        let action = crate::db::company_ops::resolve_action_authorization(
            &state.company_pool,
            id,
            &status,
            authorized_by_user_id,
            notes.as_deref(),
        ).await.map_err(|e| Error::new(e.to_string()))?;
        
        // Log this resolution in the execution logs for audit compliance
        let log_msg = format!("Action '{}' ({}) was resolved to '{}' by user.", action.action_type, action.description, status);
        let _ = crate::db::company_ops::create_execution_log(
            &state.company_pool,
            action.company_id,
            action.initiated_by_user_id,
            None,
            "Security Auditor",
            &log_msg,
            if status == "authorized" { "success" } else { "warning" },
        ).await;

        Ok(GqlActionAuthorization::from(action))
    }

    async fn log_execution(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
        agent_name: String,
        message:    String,
        log_level:  String,
    ) -> Result<GqlExecutionLog> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        let user_id    = extract_user_id(ctx)?;
        
        let el = crate::db::company_ops::create_execution_log(
            &state.company_pool,
            company_id,
            user_id,
            None,
            &agent_name,
            &message,
            &log_level,
        ).await.map_err(|e| Error::new(e.to_string()))?;
        
        Ok(GqlExecutionLog::from(el))
    }
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| Error::new(format!("Invalid UUID: {s}")))
}

fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing"))
}