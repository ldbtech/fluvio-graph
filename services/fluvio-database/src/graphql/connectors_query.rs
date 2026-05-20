//! Connector query resolvers — add these to your existing QueryRoot.

use async_graphql::*;
use uuid::Uuid;
use crate::server::AppState;
use crate::db::{connectors, resources};
use super::connectors_type::{GqlConnector, GqlConnectorResource};

/// Get all connectors for the authenticated user.
/// Optionally filter by group_id.
pub async fn get_user_connectors(
    ctx:      &Context<'_>,
    group_id: Option<String>,
) -> Result<Vec<GqlConnector>> {
    let state   = ctx.data::<AppState>()?;
    let user_id = extract_user_id(ctx)?;

    let all = connectors::get_user_connectors(&state.pool, user_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

    // Filter by group_id if provided
    let filtered = match group_id {
        Some(ref gid) => {
            let gid = Uuid::parse_str(gid)
                .map_err(|_| Error::new("invalid group_id"))?;
            all.into_iter()
                .filter(|c| c.group_id == Some(gid))
                .collect()
        }
        None => all,
    };

    Ok(filtered.into_iter().map(GqlConnector::from).collect())
}

/// Get all resources for a connector (repos, pages, etc).
pub async fn get_connector_resources(
    ctx:          &Context<'_>,
    connector_id: String,
) -> Result<Vec<GqlConnectorResource>> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    Ok(resources::get_resources(&state.pool, connector_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .into_iter()
        .map(GqlConnectorResource::from)
        .collect())
}

/// Get selected resources only (used by sync jobs).
pub async fn get_selected_resources(
    ctx:          &Context<'_>,
    connector_id: String,
) -> Result<Vec<GqlConnectorResource>> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    Ok(resources::get_selected_resources(&state.pool, connector_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .into_iter()
        .map(GqlConnectorResource::from)
        .collect())
}

pub async fn get_connector(
    ctx:          &Context<'_>,
    connector_id: String,
) -> Result<Option<GqlConnector>> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    Ok(connectors::get_connector(&state.pool, connector_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .map(GqlConnector::from))
}

fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing"))
}

