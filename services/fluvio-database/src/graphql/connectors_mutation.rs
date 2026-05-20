//! Connector mutation resolvers — add these to your existing MutationRoot.

use async_graphql::*;
use uuid::Uuid;
use serde_json::json;

use crate::server::AppState;
use crate::db::{connectors, resources};
use super::connectors_type::*;

/// Create a connector (token or OAuth).
pub async fn create_connector(
    ctx:   &Context<'_>,
    input: CreateConnectorInput,
) -> Result<GqlConnector> {
    let state   = ctx.data::<AppState>()?;
    let user_id = extract_user_id(ctx)?;

    let group_id = input.group_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| Error::new("invalid group_id"))?;

    let connector = connectors::create_connector(
        &state.pool,
        user_id,
        group_id,
        &input.kind,
        &input.auth_method,
        &input.access_token,
        None,
        None,
    ).await.map_err(|e| Error::new(e.to_string()))?;

    tracing::info!(
        user_id = %user_id,
        kind    = %input.kind,
        "connector created"
    );

    Ok(GqlConnector::from(connector))
}

/// Upsert a resource (repo, page, recording) for a connector.
/// Called by fluvio-connectors after fetching the resource list.
pub async fn upsert_resource(
    ctx:   &Context<'_>,
    input: UpsertResourceInput,
) -> Result<GqlConnectorResource> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&input.connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    let resource = resources::upsert_resource(
        &state.pool,
        connector_id,
        &input.resource_kind,
        &input.external_id,
        &input.name,
        input.description.as_deref(),
        json!({}),
    ).await.map_err(|e| Error::new(e.to_string()))?;

    Ok(GqlConnectorResource::from(resource))
}

/// Bulk select resources — user picks which repos/pages to sync.
/// Sets selected=true for listed IDs, false for all others.
pub async fn select_resources(
    ctx:   &Context<'_>,
    input: SelectResourcesInput,
) -> Result<Vec<GqlConnectorResource>> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&input.connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    resources::bulk_set_selected(
        &state.pool,
        connector_id,
        &input.external_ids,
    ).await.map_err(|e| Error::new(e.to_string()))?;

    // Return updated list
    Ok(resources::get_resources(&state.pool, connector_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .into_iter()
        .map(GqlConnectorResource::from)
        .collect())
}

/// Update connector status (called by sync jobs).
pub async fn update_connector_status(
    ctx:          &Context<'_>,
    connector_id: String,
    status:       String,
    error:        Option<String>,
) -> Result<GqlConnector> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    let connector = connectors::update_status(
        &state.pool,
        connector_id,
        &status,
        error.as_deref(),
    ).await.map_err(|e| Error::new(e.to_string()))?;

    Ok(GqlConnector::from(connector))
}

/// Mark connector last synced (called after successful sync).
pub async fn mark_synced(
    ctx:          &Context<'_>,
    connector_id: String,
) -> Result<GqlConnector> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    let connector = connectors::update_last_sync(&state.pool, connector_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

    Ok(GqlConnector::from(connector))
}

/// Update sync stats for a resource after sync.
pub async fn update_resource_sync_stats(
    ctx:          &Context<'_>,
    connector_id: String,
    external_id:  String,
    nodes_added:  i32,
) -> Result<bool> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    resources::update_sync_stats(
        &state.pool,
        connector_id,
        &external_id,
        nodes_added,
    ).await.map_err(|e| Error::new(e.to_string()))?;

    Ok(true)
}

/// Disconnect a connector — deletes it and all its resources.
pub async fn disconnect_connector(
    ctx:          &Context<'_>,
    connector_id: String,
) -> Result<bool> {
    let state        = ctx.data::<AppState>()?;
    let connector_id = Uuid::parse_str(&connector_id)
        .map_err(|_| Error::new("invalid connector_id"))?;

    let deleted = connectors::delete_connector(&state.pool, connector_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?;

    Ok(deleted)
}

fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing"))
}