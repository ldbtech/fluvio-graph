//! Connector table queries — pure data access.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::db::queries::connectors::{Connector, CREATE, GET_BY_ID,
    GET_USER_CONNECTORS, GET_GROUP_CONNECTORS,
    UPDATE_STATUS, UPDATE_LAST_SYNC, UPDATE_TOKENS, DELETE};

pub async fn create_connector(
    pool:             &PgPool,
    user_id:          Uuid,
    group_id:         Option<Uuid>,
    kind:             &str,
    auth_method:      &str,
    access_token:     &str,
    refresh_token:    Option<&str>,
    token_expires_at: Option<DateTime<Utc>>,
) -> anyhow::Result<Connector> {
    Ok(sqlx::query_as::<_, Connector>(CREATE)
        .bind(user_id)
        .bind(group_id)
        .bind(kind)
        .bind(auth_method)
        .bind(access_token)
        .bind(refresh_token)
        .bind(token_expires_at)
        .fetch_one(pool)
        .await?)
}

pub async fn get_connector(
    pool: &PgPool,
    id:   Uuid,
) -> anyhow::Result<Option<Connector>> {
    Ok(sqlx::query_as::<_, Connector>(GET_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_user_connectors(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Vec<Connector>> {
    Ok(sqlx::query_as::<_, Connector>(GET_USER_CONNECTORS)
        .bind(user_id)
        .fetch_all(pool)
        .await?)
}

pub async fn get_group_connectors(
    pool:     &PgPool,
    group_id: Uuid,
) -> anyhow::Result<Vec<Connector>> {
    Ok(sqlx::query_as::<_, Connector>(GET_GROUP_CONNECTORS)
        .bind(group_id)
        .fetch_all(pool)
        .await?)
}

pub async fn update_status(
    pool:          &PgPool,
    id:            Uuid,
    status:        &str,
    error_message: Option<&str>,
) -> anyhow::Result<Connector> {
    Ok(sqlx::query_as::<_, Connector>(UPDATE_STATUS)
        .bind(id)
        .bind(status)
        .bind(error_message)
        .fetch_one(pool)
        .await?)
}

pub async fn update_last_sync(
    pool: &PgPool,
    id:   Uuid,
) -> anyhow::Result<Connector> {
    Ok(sqlx::query_as::<_, Connector>(UPDATE_LAST_SYNC)
        .bind(id)
        .fetch_one(pool)
        .await?)
}

pub async fn update_tokens(
    pool:             &PgPool,
    id:               Uuid,
    access_token:     &str,
    refresh_token:    Option<&str>,
    token_expires_at: Option<DateTime<Utc>>,
) -> anyhow::Result<Connector> {
    Ok(sqlx::query_as::<_, Connector>(UPDATE_TOKENS)
        .bind(id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(token_expires_at)
        .fetch_one(pool)
        .await?)
}

pub async fn delete_connector(
    pool: &PgPool,
    id:   Uuid,
) -> anyhow::Result<bool> {
    let result = sqlx::query(DELETE)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}