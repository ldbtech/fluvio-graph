//! group membership queries 
use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::db::queries::members::{Member, ADD, GET, GET_ALL, UPDATE_ROLE, REMOVE, OWNER_COUNT};

pub async fn add_member(
    pool: &PgPool,
    group_id: Uuid,
    user_id: Uuid,
    role: &str,
    invited_by: Option<Uuid>,
) -> anyhow::Result<Member> {
    Ok(sqlx::query_as::<_, Member>(ADD)
        .bind(group_id)
        .bind(user_id)
        .bind(role)
        .bind(invited_by)
        .fetch_one(pool)
        .await?)
}

pub async fn get_member(
    pool: &PgPool,
    group_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<Option<Member>> {
    Ok(sqlx::query_as::<_, Member>(GET)
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_group_members(
    pool: &PgPool,
    group_id: Uuid,
) -> anyhow::Result<Vec<Member>> {
    Ok(sqlx::query_as::<_, Member>(GET_ALL)
        .bind(group_id)
        .fetch_all(pool)
        .await?)
}

pub async fn update_member_role(
    pool: &PgPool,
    group_id: Uuid,
    user_id: Uuid,
    role: &str,
) -> anyhow::Result<Member> {
    Ok(sqlx::query_as::<_, Member>(UPDATE_ROLE)
        .bind(group_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(pool)
        .await?)
}

pub async fn remove_member(
    pool: &PgPool,
    group_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<bool> {
    let result = sqlx::query(REMOVE)
        .bind(group_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn get_group_owner_count(
    pool: &PgPool,
    group_id: Uuid,
) -> anyhow::Result<i64> {
    let row = sqlx::query(OWNER_COUNT)
    .bind(group_id)
    .fetch_one(pool)
    .await?;
    Ok(row.try_get::<i64, _>("count").unwrap_or(0))
}