//! Connector resource table queries — repos, pages, recordings.

use sqlx::PgPool;
use uuid::Uuid;
use serde_json::Value;

use crate::db::queries::resources::{ConnectorResource, UPSERT,
    GET_BY_CONNECTOR, GET_SELECTED, SET_SELECTED,
    UPDATE_SYNC_STATS, BULK_SET_SELECTED};

/// Insert or update a resource (repo, page, recording).
/// On conflict updates name, description, and meta.
pub async fn upsert_resource(
    pool:          &PgPool,
    connector_id:  Uuid,
    resource_kind: &str,
    external_id:   &str,
    name:          &str,
    description:   Option<&str>,
    meta:          Value,
) -> anyhow::Result<ConnectorResource> {
    Ok(sqlx::query_as::<_, ConnectorResource>(UPSERT)
        .bind(connector_id)
        .bind(resource_kind)
        .bind(external_id)
        .bind(name)
        .bind(description)
        .bind(meta)
        .fetch_one(pool)
        .await?)
}

/// Get all resources for a connector.
pub async fn get_resources(
    pool:         &PgPool,
    connector_id: Uuid,
) -> anyhow::Result<Vec<ConnectorResource>> {
    Ok(sqlx::query_as::<_, ConnectorResource>(GET_BY_CONNECTOR)
        .bind(connector_id)
        .fetch_all(pool)
        .await?)
}

/// Get only selected resources (for sync).
pub async fn get_selected_resources(
    pool:         &PgPool,
    connector_id: Uuid,
) -> anyhow::Result<Vec<ConnectorResource>> {
    Ok(sqlx::query_as::<_, ConnectorResource>(GET_SELECTED)
        .bind(connector_id)
        .fetch_all(pool)
        .await?)
}

/// Select or deselect a single resource.
pub async fn set_selected(
    pool:         &PgPool,
    connector_id: Uuid,
    external_id:  &str,
    selected:     bool,
) -> anyhow::Result<ConnectorResource> {
    Ok(sqlx::query_as::<_, ConnectorResource>(SET_SELECTED)
        .bind(connector_id)
        .bind(selected)
        .bind(external_id)
        .fetch_one(pool)
        .await?)
}

/// Bulk select resources — sets selected=true for listed IDs,
/// selected=false for all others in this connector.
pub async fn bulk_set_selected(
    pool:         &PgPool,
    connector_id: Uuid,
    external_ids: &[String],
) -> anyhow::Result<()> {
    sqlx::query(BULK_SET_SELECTED)
        .bind(connector_id)
        .bind(external_ids)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update sync stats after a successful sync of one resource.
pub async fn update_sync_stats(
    pool:           &PgPool,
    connector_id:   Uuid,
    external_id:    &str,
    nodes_added:    i32,
) -> anyhow::Result<()> {
    sqlx::query(UPDATE_SYNC_STATS)
        .bind(connector_id)
        .bind(nodes_added)
        .bind(external_id)
        .execute(pool)
        .await?;
    Ok(())
}