//! Mutation resolvers for fluvio-database.

use async_graphql::*;
use uuid::Uuid;
use chrono::Utc;
use chrono::DateTime;

use crate::server::AppState;
use fluvio_database::db::{users, groups, members, invites, queue, companies, teams, queries::users::User};
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
        let team_id  = input.team_id.map(|tid| parse_uuid(&tid)).transpose()?;
        Ok(fluvio_database::db::workspaces::create_workspace(
            &state.pool,
            owner_id,
            &input.name,
            input.is_public,
            team_id,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlWorkspace::from)?)
    }

    async fn update_workspace(
        &self,
        ctx:   &Context<'_>,
        input: UpdateWorkspaceInput,
    ) -> Result<GqlWorkspace> {
        let state   = ctx.data::<AppState>()?;
        let id      = parse_uuid(&input.id)?;
        let team_id = input.team_id.map(|tid| parse_uuid(&tid)).transpose()?;
        Ok(fluvio_database::db::workspaces::update_workspace(
            &state.pool,
            id,
            input.name.as_deref(),
            input.is_public,
            team_id,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlWorkspace::from)?)
    }

    async fn create_planner_approval(
        &self,
        ctx:   &Context<'_>,
        input: CreatePlannerApprovalInput,
    ) -> Result<GqlPlannerApproval> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&input.workspace_id)?;
        let suggested_by = parse_uuid(&input.suggested_by)?;
        let details: serde_json::Value = serde_json::from_str(&input.change_details)
            .map_err(|_| Error::new("Invalid JSON in change_details"))?;

        Ok(fluvio_database::db::planner_approvals::create_planner_approval(
            &state.pool,
            workspace_id,
            suggested_by,
            &input.change_type,
            details,
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlPlannerApproval::from)?)
    }

    async fn review_planner_approval(
        &self,
        ctx:   &Context<'_>,
        input: ReviewPlannerApprovalInput,
    ) -> Result<GqlPlannerApproval> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&input.id)?;
        
        Ok(fluvio_database::db::planner_approvals::review_planner_approval(
            &state.pool,
            id,
            &input.status,
            input.review_note.as_deref(),
        ).await.map_err(|e| Error::new(e.to_string()))
        .map(GqlPlannerApproval::from)?)
    }


    async fn delete_workspace(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        fluvio_database::db::workspaces::delete_workspace(&state.pool, id).await
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
            
        let share = fluvio_database::db::workspaces::share_workspace(&state.pool, workspace_id, user.id).await
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
        fluvio_database::db::workspaces::unshare_workspace(&state.pool, workspace_id, user_id).await
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

    async fn toggle_workflow_enabled(
        &self,
        ctx:        &Context<'_>,
        id:         String,
        is_enabled: bool,
    ) -> Result<GqlTeamWorkflow> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        Ok(teams::toggle_workflow_enabled(&state.pool, id, is_enabled).await
            .map_err(|e| Error::new(e.to_string()))
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
        
        fluvio_database::db::workspaces::verify_workspace_access(&state.pool, workspace_id, user_id).await
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
        
        fluvio_database::db::workspaces::verify_workspace_access(&state.pool, workspace_id, user_id).await
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
        
        let action = fluvio_database::db::company_ops::resolve_action_authorization(
            &state.company_pool,
            id,
            &status,
            authorized_by_user_id,
            notes.as_deref(),
        ).await.map_err(|e| Error::new(e.to_string()))?;
        
        // Log this resolution in the execution logs for audit compliance
        let log_msg = format!("Action '{}' ({}) was resolved to '{}' by user.", action.action_type, action.description, status);
        let _ = fluvio_database::db::company_ops::create_execution_log(
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
        
        let el = fluvio_database::db::company_ops::create_execution_log(
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

    async fn register_employee(
        &self,
        ctx:          &Context<'_>,
        firebase_uid: String,
        email:        String,
        display_name: String,
        role:         String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;

        // Fetch the admin's company_id
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company_id = admin_user.company_id
            .ok_or_else(|| Error::new("You must belong to a company to register employees"))?;

        // Check if admin is indeed an admin (createdBy matches or role is admin)
        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can register employees"));
        }

        // Create the user in database with ReadOnlyAccess default policy
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (firebase_uid, email, display_name, company_email, company_id, role, must_change_password, policies)
             VALUES ($1, $2, $3, $2, $4, $5, TRUE, ARRAY['ReadOnlyAccess'])
             ON CONFLICT (firebase_uid) DO UPDATE
               SET email = EXCLUDED.email,
                   display_name = EXCLUDED.display_name,
                   company_email = EXCLUDED.company_email,
                   company_id = EXCLUDED.company_id,
                   role = EXCLUDED.role,
                   must_change_password = TRUE,
                   updated_at = now()
             RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                       role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at"
        )
        .bind(&firebase_uid)
        .bind(&email)
        .bind(&display_name)
        .bind(company_id)
        .bind(&role)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        // Add the employee to the "General" team of the company automatically
        if let Ok(teams) = teams::get_company_teams(&state.pool, company_id).await {
            if let Some(general_team) = teams.iter().find(|t| t.name == "General") {
                let _ = teams::add_team_member(&state.pool, general_team.id, user.id, "member").await;
            }
        }

        Ok(GqlUser::from(user))
    }

    async fn attach_user_policy(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
        policy:  String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id = user.company_id
            .ok_or_else(|| Error::new("User does not belong to any company"))?;

        // Check admin permissions
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can attach policies"));
        }

        // Attach policy if not already attached
        let updated = sqlx::query_as::<_, User>(
            "UPDATE users 
             SET policies = ARRAY(
               SELECT DISTINCT unnest(array_append(policies, $2))
             ), updated_at = now() 
             WHERE id = $1 
             RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                       role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at"
        )
        .bind(target_user_id)
        .bind(&policy)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlUser::from(updated))
    }

    async fn detach_user_policy(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
        policy:  String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id = user.company_id
            .ok_or_else(|| Error::new("User does not belong to any company"))?;

        // Check admin permissions
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can detach policies"));
        }

        // Detach policy
        let updated = sqlx::query_as::<_, User>(
            "UPDATE users 
             SET policies = array_remove(policies, $2), updated_at = now() 
             WHERE id = $1 
             RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                       role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at"
        )
        .bind(target_user_id)
        .bind(&policy)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlUser::from(updated))
    }

    async fn attach_user_twin_role(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
        role:    String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id = user.company_id
            .ok_or_else(|| Error::new("User does not belong to any company"))?;

        // Check admin permissions
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can assign twin roles"));
        }

        // Attach role if not already attached
        let updated = sqlx::query_as::<_, User>(
            "UPDATE users 
             SET assigned_agent_roles = ARRAY(
               SELECT DISTINCT unnest(array_append(assigned_agent_roles, $2))
             ), updated_at = now() 
             WHERE id = $1 
             RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                       role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at"
        )
        .bind(target_user_id)
        .bind(&role)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlUser::from(updated))
    }

    async fn detach_user_twin_role(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
        role:    String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id = user.company_id
            .ok_or_else(|| Error::new("User does not belong to any company"))?;

        // Check admin permissions
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can detach twin roles"));
        }

        // Detach role
        let updated = sqlx::query_as::<_, User>(
            "UPDATE users 
             SET assigned_agent_roles = array_remove(assigned_agent_roles, $2), updated_at = now() 
             WHERE id = $1 
             RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                       role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at"
        )
        .bind(target_user_id)
        .bind(&role)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlUser::from(updated))
    }

    async fn draft_twin_role(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
    ) -> Result<String> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // 1. Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let mut company_name = "Personal Workspace".to_string();
        let mut squads_str = "None (Personal Workspace)".to_string();
        let mut connectors_str = "None".to_string();

        if let Some(company_id) = user.company_id {
            // 2. Check admin permissions
            let admin_user = users::get_user_by_id(&state.pool, admin_id).await
                .map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("Admin user not found"))?;

            let company = companies::get_company(&state.pool, company_id).await
                .map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("Company not found"))?;

            if company.created_by != admin_id && admin_user.role != "admin" && admin_id != target_user_id {
                return Err(Error::new("Only company admins can draft twin roles"));
            }

            company_name = company.name;

            // 3. Fetch user's squads/teams
            let squads: Vec<teams::Team> = sqlx::query_as::<_, teams::Team>(
                "SELECT t.id, t.company_id, t.name, t.description, t.created_at, t.updated_at 
                 FROM teams t
                 INNER JOIN team_members tm ON t.id = tm.team_id
                 WHERE tm.user_id = $1"
            )
            .bind(target_user_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

            if !squads.is_empty() {
                squads_str = squads.iter()
                    .map(|s| format!("{} ({})", s.name, s.description.as_deref().unwrap_or("No description")))
                    .collect::<Vec<String>>()
                    .join(", ");
            }

            // 4. Fetch company's active connectors
            let connectors: Vec<String> = sqlx::query_scalar::<_, String>(
                "SELECT DISTINCT kind::text FROM connectors WHERE user_id IN (
                   SELECT id FROM users WHERE company_id = $1
                 )"
            )
            .bind(company_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

            if !connectors.is_empty() {
                connectors_str = connectors.join(", ");
            }
        } else {
            // Personal user
            if admin_id != target_user_id {
                return Err(Error::new("Only the user can draft their own personal twin manifest"));
            }

            // Fetch user's active connectors
            let connectors: Vec<String> = sqlx::query_scalar::<_, String>(
                "SELECT DISTINCT kind::text FROM connectors WHERE user_id = $1"
            )
            .bind(target_user_id)
            .fetch_all(&state.pool)
            .await
            .map_err(|e| Error::new(e.to_string()))?;

            if !connectors.is_empty() {
                connectors_str = connectors.join(", ");
            }
        }

        // 5. Query Anthropic API to generate draft
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::new("ANTHROPIC_API_KEY environment variable is not set"))?;

        let display_name = user.display_name.unwrap_or_else(|| user.email.clone().unwrap_or_else(|| "Employee".to_string()));
        let user_position = user.role; // position/role
        let policies_str = if user.policies.is_empty() {
            "None (ReadOnlyAccess)".to_string()
        } else {
            user.policies.join(", ")
        };

        let system_prompt = if user.company_id.is_some() {
            format!(
                "You are the Enterprise Architect for the company '{}'. Your job is to draft a custom TwinAgentRole markdown specification for an employee.

Analyze the user's role/position, their squad, their specific IAM permission policies, and the company's active integrations/tools to output a highly personalized agent role description, directive, permission boundaries, and a checklist of cooperative tasks. Make sure the twin agent's capabilities do not exceed the user's IAM permission policies.

Your response MUST contain ONLY the raw markdown content. Do NOT wrap it in markdown code blocks like ```markdown ... ```. Do NOT include any introduction, conversational filler, or wrap-up commentary. Start immediately with '# Agent Role: [Name]'.",
                company_name
            )
        } else {
            "You are the Personal AI Architect. Your job is to draft a custom TwinAgentRole markdown specification for an individual user in their personal workspace.

Analyze the user's role/position, their specific IAM permission policies, and their active integrations/tools to output a highly personalized agent role description, directive, permission boundaries, and a checklist of cooperative tasks. Make sure the twin agent's capabilities do not exceed the user's IAM permission policies.

Your response MUST contain ONLY the raw markdown content. Do NOT wrap it in markdown code blocks like ```markdown ... ```. Do NOT include any introduction, conversational filler, or wrap-up commentary. Start immediately with '# Agent Role: [Name]'.".to_string()
        };

        let user_message = if user.company_id.is_some() {
            format!(
                "Company Name: {}\nEmployee Name: {}\nEmployee Position/Role: {}\nSquads: {}\nIAM Permission Policies: {}\nActive Company Tools/Connectors: {}\n\nDraft a highly tailored twin agent role.",
                company_name,
                display_name,
                user_position,
                squads_str,
                policies_str,
                connectors_str
            )
        } else {
            format!(
                "Company Name: Personal Workspace\nUser Name: {}\nPosition/Role: {}\nSquads: {}\nIAM Permission Policies: {}\nActive Personal Tools/Connectors: {}\n\nDraft a highly tailored twin agent role.",
                display_name,
                user_position,
                squads_str,
                policies_str,
                connectors_str
            )
        };

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": user_message }
            ]
        });

        let resp: serde_json::Value = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::new(format!("Failed to connect to Anthropic API: {}", e)))?
            .json()
            .await
            .map_err(|e| Error::new(format!("Failed to parse Anthropic response: {}", e)))?;

        let answer = resp["content"][0]["text"]
            .as_str()
            .ok_or_else(|| {
                let err_msg = resp["error"]["message"].as_str().unwrap_or("Unknown error");
                Error::new(format!("Anthropic API error: {}", err_msg))
            })?
            .to_string();

        Ok(answer)
    }

    async fn save_user_twin_manifest(
        &self,
        ctx:      &Context<'_>,
        user_id:  String,
        manifest: String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // 1. Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        if let Some(company_id) = user.company_id {
            // 2. Check admin permissions
            let admin_user = users::get_user_by_id(&state.pool, admin_id).await
                .map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("Admin user not found"))?;

            let company = companies::get_company(&state.pool, company_id).await
                .map_err(|e| Error::new(e.to_string()))?
                .ok_or_else(|| Error::new("Company not found"))?;

            if company.created_by != admin_id && admin_user.role != "admin" && admin_id != target_user_id {
                return Err(Error::new("Only company admins can save twin manifests"));
            }
        } else {
            // Personal user
            if admin_id != target_user_id {
                return Err(Error::new("Only the user can save their own personal twin manifest"));
            }
        }

        // 3. Save twin manifest
        let updated = sqlx::query_as::<_, User>(
            "UPDATE users 
             SET twin_manifest = $2, updated_at = now() 
             WHERE id = $1 
             RETURNING id, firebase_uid, email, display_name, avatar_url, company_email, company_id,
                       role, must_change_password, policies, assigned_agent_roles, twin_manifest, created_at, updated_at"
        )
        .bind(target_user_id)
        .bind(&manifest)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlUser::from(updated))
    }

    async fn complete_password_reset(
        &self,
        ctx: &Context<'_>,
    ) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;

        sqlx::query(
            "UPDATE users SET must_change_password = FALSE, updated_at = now() WHERE id = $1"
        )
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(true)
    }

    async fn update_user_role(
        &self,
        ctx:      &Context<'_>,
        user_id:  String,
        new_role: String,
    ) -> Result<GqlUser> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id = user.company_id
            .ok_or_else(|| Error::new("User does not belong to any company"))?;

        // Check admin permissions
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can change roles"));
        }

        // Update role
        let updated = sqlx::query_as::<_, User>(
            "UPDATE users SET role = $2, updated_at = now() WHERE id = $1 RETURNING *"
        )
        .bind(target_user_id)
        .bind(&new_role)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(GqlUser::from(updated))
    }

    async fn remove_user_from_company(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
    ) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        let admin_id = extract_user_id(ctx)?;
        let target_user_id = parse_uuid(&user_id)?;

        // Fetch target user
        let user = users::get_user_by_id(&state.pool, target_user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id = user.company_id
            .ok_or_else(|| Error::new("User does not belong to any company"))?;

        // Check admin permissions
        let admin_user = users::get_user_by_id(&state.pool, admin_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Admin user not found"))?;

        let company = companies::get_company(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != admin_id && admin_user.role != "admin" {
            return Err(Error::new("Only company admins can remove employees"));
        }

        if user.id == company.created_by {
            return Err(Error::new("Cannot remove the company owner/creator"));
        }

        // Set company_id to NULL and company_email to NULL (or just clear company details)
        sqlx::query(
            "UPDATE users SET company_id = NULL, company_email = NULL, updated_at = now() WHERE id = $1"
        )
        .bind(target_user_id)
        .execute(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        // Also remove user from all company squads
        sqlx::query(
            "DELETE FROM team_members WHERE user_id = $1 AND team_id IN (SELECT id FROM teams WHERE company_id = $2)"
        )
        .bind(target_user_id)
        .bind(company_id)
        .execute(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(true)
    }

    async fn delete_company(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
    ) -> Result<bool> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        
        let user = users::get_user_by_id(&state.pool, user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("User not found"))?;

        let company_id_parsed = parse_uuid(&company_id)?;

        if user.company_id != Some(company_id_parsed) {
            return Err(Error::new("Permission denied: You do not belong to this company"));
        }

        let company = companies::get_company(&state.pool, company_id_parsed).await
            .map_err(|e| Error::new(e.to_string()))?
            .ok_or_else(|| Error::new("Company not found"))?;

        if company.created_by != user_id && user.role != "admin" {
            return Err(Error::new("Permission denied: Only company owners or admins can delete the company"));
        }

        // Delete telemetry data from the company pool
        fluvio_database::db::company_ops::delete_company_data(&state.company_pool, company_id_parsed).await
            .map_err(|e| Error::new(e.to_string()))?;

        // Delete company from core pool
        companies::delete_company(&state.pool, company_id_parsed).await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(true)
     }

    async fn delete_user(
        &self,
        ctx: &Context<'_>,
    ) -> Result<bool> {
        let state = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;

        // Check if user is a company owner (createdBy of any company)
        let is_owner: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM companies WHERE created_by = $1)"
        )
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        if is_owner {
            return Err(Error::new("Cannot delete account: You are the owner of a company. Please delete the company or transfer ownership first."));
        }

        // Call database helper
        users::delete_user(&state.pool, user_id).await
            .map_err(|e| Error::new(e.to_string()))?;

        Ok(true)
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