//! Query resolvers for fluvio-database.

use async_graphql::*;
use uuid::Uuid;
use crate::server::AppState;
use fluvio_database::db::{users, groups, members, invites, queue, companies, teams};
use crate::graphql::types::*;

use crate::graphql::connectors_type;
use crate::graphql::connectors_query;

use crate::graphql::llm_providers_type;
use crate::graphql::llm_providers_query;

pub struct QueryRoot;

#[Object(name = "Query")]
impl QueryRoot {

    async fn get_user(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<Option<GqlUser>> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        Ok(users::get_user_by_id(&state.pool, id).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlUser::from))
    }

    async fn get_user_by_firebase_uid(
        &self,
        ctx:          &Context<'_>,
        firebase_uid: String,
    ) -> Result<Option<GqlUser>> {
        let state = ctx.data::<AppState>()?;
        Ok(users::get_user_by_firebase_uid(&state.pool, &firebase_uid).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlUser::from))
    }

    async fn get_group(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<Option<GqlGroup>> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        Ok(groups::get_group(&state.pool, id).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlGroup::from))
    }

    async fn get_user_groups(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
    ) -> Result<Vec<GqlGroup>> {
        let state   = ctx.data::<AppState>()?;
        let user_id = parse_uuid(&user_id)?;
        Ok(groups::get_user_groups(&state.pool, user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlGroup::from).collect())
    }

    async fn get_member(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
        user_id:  String,
    ) -> Result<Option<GqlMember>> {
        let state    = ctx.data::<AppState>()?;
        let group_id = parse_uuid(&group_id)?;
        let user_id  = parse_uuid(&user_id)?;
        Ok(members::get_member(&state.pool, group_id, user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlMember::from))
    }

    async fn get_group_members(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
    ) -> Result<Vec<GqlMember>> {
        let state    = ctx.data::<AppState>()?;
        let group_id = parse_uuid(&group_id)?;
        Ok(members::get_group_members(&state.pool, group_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlMember::from).collect())
    }

    async fn get_invite_by_token(
        &self,
        ctx:   &Context<'_>,
        token: String,
    ) -> Result<Option<GqlInvite>> {
        let state = ctx.data::<AppState>()?;
        Ok(invites::get_invite_by_token(&state.pool, &token).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlInvite::from))
    }

    async fn get_group_invites(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
    ) -> Result<Vec<GqlInvite>> {
        let state    = ctx.data::<AppState>()?;
        let group_id = parse_uuid(&group_id)?;
        Ok(invites::get_group_invites(&state.pool, group_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlInvite::from).collect())
    }

    async fn get_pending_queue(
        &self,
        ctx:      &Context<'_>,
        group_id: String,
    ) -> Result<Vec<GqlQueueItem>> {
        let state    = ctx.data::<AppState>()?;
        let group_id = parse_uuid(&group_id)?;
        Ok(queue::get_pending(&state.pool, group_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlQueueItem::from).collect())
    }

    async fn get_user_contributions(
        &self,
        ctx:            &Context<'_>,
        group_id:       String,
        contributed_by: String,
    ) -> Result<Vec<GqlQueueItem>> {
        let state          = ctx.data::<AppState>()?;
        let group_id       = parse_uuid(&group_id)?;
        let contributed_by = parse_uuid(&contributed_by)?;
        Ok(queue::get_user_contributions(&state.pool, group_id, contributed_by).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlQueueItem::from).collect())
    }

    async fn get_user_connectors(
        &self, ctx: &Context<'_>, group_id: Option<String>,
    ) -> Result<Vec<connectors_type::GqlConnector>> {
        connectors_query::get_user_connectors(ctx, group_id).await
    }
    
    async fn get_connector_resources(
        &self, ctx: &Context<'_>, connector_id: String,
    ) -> Result<Vec<connectors_type::GqlConnectorResource>> {
        connectors_query::get_connector_resources(ctx, connector_id).await
    }
    
    async fn get_selected_resources(
        &self, ctx: &Context<'_>, connector_id: String,
    ) -> Result<Vec<connectors_type::GqlConnectorResource>> {
        connectors_query::get_selected_resources(ctx, connector_id).await
    }

    async fn get_connector(
        &self, ctx: &Context<'_>, connector_id: String,
    ) -> Result<Option<connectors_type::GqlConnector>> {
        connectors_query::get_connector(ctx, connector_id).await
    }

    async fn get_user_llm_providers(
        &self, ctx: &Context<'_>, group_id: Option<String>,
    ) -> Result<Vec<llm_providers_type::GqlLlmProvider>> {
        llm_providers_query::get_user_llm_providers(ctx, group_id).await
    }

    async fn my_workspaces(
        &self,
        ctx:     &Context<'_>,
        user_id: String,
    ) -> Result<Vec<GqlWorkspace>> {
        let state   = ctx.data::<AppState>()?;
        let user_id = parse_uuid(&user_id)?;
        Ok(fluvio_database::db::workspaces::get_user_workspaces(&state.pool, user_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlWorkspace::from).collect())
    }

    async fn workspace_shares(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
    ) -> Result<Vec<GqlWorkspaceShare>> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        Ok(fluvio_database::db::workspaces::get_workspace_shares(&state.pool, workspace_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlWorkspaceShare::from).collect())
    }

    async fn planner_approvals(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
    ) -> Result<Vec<GqlPlannerApproval>> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        Ok(fluvio_database::db::planner_approvals::get_workspace_approvals(&state.pool, workspace_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlPlannerApproval::from).collect())
    }

    // ── Company & Teams ──────────────────────────────────────────────────────────

    async fn get_company(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<Option<GqlCompany>> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        Ok(companies::get_company(&state.pool, id).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlCompany::from))
    }

    async fn get_pending_company_invites(
        &self,
        ctx:   &Context<'_>,
        email: String,
    ) -> Result<Vec<GqlCompanyInvite>> {
        let state = ctx.data::<AppState>()?;
        Ok(companies::get_pending_company_invites_by_email(&state.pool, &email).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlCompanyInvite::from).collect())
    }

    async fn get_company_teams(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
    ) -> Result<Vec<GqlTeam>> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        Ok(teams::get_company_teams(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlTeam::from).collect())
    }

    async fn get_company_users(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
    ) -> Result<Vec<GqlUser>> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        Ok(users::get_company_users(&state.pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlUser::from).collect())
    }

    async fn get_team(
        &self,
        ctx: &Context<'_>,
        id:  String,
    ) -> Result<Option<GqlTeam>> {
        let state = ctx.data::<AppState>()?;
        let id    = parse_uuid(&id)?;
        Ok(teams::get_team(&state.pool, id).await
            .map_err(|e| Error::new(e.to_string()))?
            .map(GqlTeam::from))
    }

    async fn get_team_members(
        &self,
        ctx:     &Context<'_>,
        team_id: String,
    ) -> Result<Vec<GqlTeamMember>> {
        let state   = ctx.data::<AppState>()?;
        let team_id = parse_uuid(&team_id)?;
        Ok(teams::get_team_members(&state.pool, team_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlTeamMember::from).collect())
    }

    async fn get_team_workflows(
        &self,
        ctx:     &Context<'_>,
        team_id: String,
    ) -> Result<Vec<GqlTeamWorkflow>> {
        let state   = ctx.data::<AppState>()?;
        let team_id = parse_uuid(&team_id)?;
        Ok(teams::get_team_workflows(&state.pool, team_id).await
            .map_err(|e| Error::new(e.to_string()))?
            .into_iter().map(GqlTeamWorkflow::from).collect())
    }

    async fn planner_chat_history(
        &self,
        ctx:          &Context<'_>,
        workspace_id: String,
    ) -> Result<Vec<GqlPlannerChatMessage>> {
        let state        = ctx.data::<AppState>()?;
        let workspace_id = parse_uuid(&workspace_id)?;
        let user_id      = extract_user_id(ctx)?;
        
        fluvio_database::db::workspaces::verify_workspace_access(&state.pool, workspace_id, user_id).await
            .map_err(|e| Error::new(format!("Access Denied: {}", e)))?;

        let messages = sqlx::query_as::<_, GqlPlannerChatMessage>(
            "SELECT id::text, workspace_id::text, sender, content, created_at::text
             FROM planner_chat_messages
             WHERE workspace_id = $1
             ORDER BY created_at ASC"
        )
        .bind(workspace_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        Ok(messages)
    }

    async fn company_execution_logs(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
        limit:      Option<i32>,
    ) -> Result<Vec<GqlExecutionLog>> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        let limit_val  = limit.unwrap_or(50) as i64;
        
        let logs = fluvio_database::db::company_ops::get_execution_logs(&state.company_pool, company_id, limit_val).await
            .map_err(|e| Error::new(e.to_string()))?;
            
        Ok(logs.into_iter().map(GqlExecutionLog::from).collect())
    }

    async fn company_action_authorizations(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
        status:     Option<String>,
    ) -> Result<Vec<GqlActionAuthorization>> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        
        let actions = fluvio_database::db::company_ops::get_action_authorizations(&state.company_pool, company_id, status.as_deref()).await
            .map_err(|e| Error::new(e.to_string()))?;
            
        Ok(actions.into_iter().map(GqlActionAuthorization::from).collect())
    }

    async fn company_document_reconciliations(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
    ) -> Result<Vec<GqlDocumentReconciliation>> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        
        let reconciliations = fluvio_database::db::company_ops::get_document_reconciliations(&state.company_pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?;
            
        Ok(reconciliations.into_iter().map(GqlDocumentReconciliation::from).collect())
    }

    async fn company_pipeline_runs(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
    ) -> Result<Vec<GqlPipelineRun>> {
        let state      = ctx.data::<AppState>()?;
        let company_id = parse_uuid(&company_id)?;
        
        let runs = fluvio_database::db::company_ops::get_pipeline_runs(&state.company_pool, company_id).await
            .map_err(|e| Error::new(e.to_string()))?;
            
        Ok(runs.into_iter().map(GqlPipelineRun::from).collect())
    }

    async fn download_company_data(
        &self,
        ctx:        &Context<'_>,
        company_id: String,
    ) -> Result<String> {
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
            return Err(Error::new("Permission denied: Only company owners or admins can download company data"));
        }

        // Fetch all main DB tables as a single JSON object using postgres json_build_object
        let main_json: serde_json::Value = sqlx::query_scalar(
            "SELECT json_build_object(
                'company', (SELECT row_to_json(c) FROM (SELECT id, name, website, linkedin_url, twitter_url, github_url, created_by, created_at, updated_at FROM companies WHERE id = $1) c),
                'users', COALESCE((SELECT json_agg(u) FROM (SELECT id, email, display_name, avatar_url, company_email, role, policies, assigned_agent_roles, created_at FROM users WHERE company_id = $1) u), '[]'::json),
                'invites', COALESCE((SELECT json_agg(ci) FROM (SELECT id, invited_by, email, role, expires_at, accepted_at, created_at FROM company_invites WHERE company_id = $1) ci), '[]'::json),
                'teams', COALESCE((SELECT json_agg(t) FROM (SELECT id, name, description, created_at, updated_at FROM teams WHERE company_id = $1) t), '[]'::json),
                'team_members', COALESCE((SELECT json_agg(tm) FROM (SELECT id, team_id, user_id, role, joined_at FROM team_members WHERE team_id IN (SELECT id FROM teams WHERE company_id = $1)) tm), '[]'::json),
                'workflows', COALESCE((SELECT json_agg(tw) FROM (SELECT id, team_id, name, description, steps, created_by, created_at, updated_at FROM team_workflows WHERE team_id IN (SELECT id FROM teams WHERE company_id = $1)) tw), '[]'::json)
            )"
        )
        .bind(company_id_parsed)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        // Fetch telemetry DB tables as a single JSON object using company_pool
        let telemetry_json: serde_json::Value = sqlx::query_scalar(
            "SELECT json_build_object(
                'execution_logs', COALESCE((SELECT json_agg(el) FROM (SELECT id, initiated_by_user_id, initiated_by_twin_id, agent_name, message, log_level, timestamp FROM execution_logs WHERE company_id = $1) el), '[]'::json),
                'action_authorizations', COALESCE((SELECT json_agg(aa) FROM (SELECT id, action_type, description, severity, initiated_by_user_id, status, authorized_by_user_id, notes, created_at, resolved_at FROM action_authorizations WHERE company_id = $1) aa), '[]'::json),
                'document_reconciliations', COALESCE((SELECT json_agg(dr) FROM (SELECT id, title, description, source_a, source_b, resolved_to, time_ago, created_at FROM document_reconciliations WHERE company_id = $1) dr), '[]'::json),
                'pipeline_runs', COALESCE((SELECT json_agg(pr) FROM (SELECT id, name, agent_name, status, progress, detail, started_at, updated_at FROM pipeline_runs WHERE company_id = $1) pr), '[]'::json)
            )"
        )
        .bind(company_id_parsed)
        .fetch_one(&state.company_pool)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

        let mut combined = main_json;
        if let Some(obj) = combined.as_object_mut() {
            if let Some(tel_obj) = telemetry_json.as_object() {
                for (k, v) in tel_obj {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }

        let result_str = serde_json::to_string_pretty(&combined)
            .map_err(|e| Error::new(format!("Failed to serialize company data: {}", e)))?;

        Ok(result_str)
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