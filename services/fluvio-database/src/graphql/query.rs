//! Query resolvers for fluvio-database.

use async_graphql::*;
use uuid::Uuid;
use crate::server::AppState;
use crate::db::{users, groups, members, invites, queue};
use crate::graphql::types::*;

use crate::graphql::connectors_type;
use crate::graphql::connectors_query;

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
}

fn parse_uuid(s: &str) -> Result<Uuid> {
    Uuid::parse_str(s).map_err(|_| Error::new(format!("Invalid UUID: {s}")))
}