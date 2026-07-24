//! Approval queue queries - pure data access no business logic. 
use sqlx::PgPool;
use uuid::Uuid;
use crate::db::queries::queue::{QueueItem, SUBMIT, GET_PENDING, GET_BY_ID, UPDATE_STATUS, GET_USER_CONTRIBUTIONS};
use chrono::{DateTime, Utc};

pub async fn submit_to_queue(
    pool: &PgPool,
    group_id: Uuid,
    contributed_by: Uuid,
    kind: &str,
    surreal_node_id: &str,
) -> anyhow::Result<QueueItem> {
    Ok(sqlx::query_as(SUBMIT)
        .bind(group_id)
        .bind(contributed_by)
        .bind(kind)
        .bind(surreal_node_id)
        .fetch_one(pool)
        .await?)
}

pub async fn get_pending(
    pool: &PgPool,
    group_id: Uuid,
) -> anyhow::Result<Vec<QueueItem>> {
    Ok(sqlx::query_as(GET_PENDING)
        .bind(group_id)
        .fetch_all(pool)
        .await?)
}

pub async fn get_queue_item(
    pool: &PgPool,
    id: Uuid,
) -> anyhow::Result<Option<QueueItem>> {
    Ok(sqlx::query_as(GET_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn update_queue_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    reviewed_by: Uuid,
    review_note: Option<&str>,
) -> anyhow::Result<QueueItem> {
    Ok(sqlx::query_as(UPDATE_STATUS)
        .bind(id)
        .bind(status)
        .bind(reviewed_by)
        .bind(review_note)
        .fetch_one(pool)
        .await?)
}

pub async fn get_user_contributions(
    pool: &PgPool,
    group_id: Uuid,
    contributed_by: Uuid,
) -> anyhow::Result<Vec<QueueItem>> {
    Ok(sqlx::query_as(GET_USER_CONTRIBUTIONS)
        .bind(group_id)
        .bind(contributed_by)
        .fetch_all(pool)
        .await?)
}