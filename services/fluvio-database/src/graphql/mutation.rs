//! Mutation resolvers for fluvio-database.

use async_graphql::*;
use uuid::Uuid;
use chrono::Utc;

use crate::server::AppState;
use crate::db::{users, groups, members, invites, queue};
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
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| Error::new(format!("Invalid UUID: {s}")))
}