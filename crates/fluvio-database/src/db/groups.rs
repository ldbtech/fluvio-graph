//! groups.rs table queries - pure data access no business logic. 

use sqlx::PgPool;
use uuid::Uuid;
use crate::db::queries::groups::{Group, CREATE, GET_BY_ID, GET_USER_GROUPS, UPDATE};

pub async fn create_group(pool: &PgPool, name: &str, description: Option<&str>, created_by: Uuid) -> anyhow::Result<Group> {
    Ok(sqlx::query_as::<_, Group>(CREATE)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .fetch_one(pool)
        .await?)
}

pub async fn get_group(
    pool: &PgPool,
    id: Uuid,
) -> anyhow::Result<Option<Group>> {
    Ok(sqlx::query_as::<_, Group>(GET_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_user_groups(
    pool: &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Vec<Group>> {
    Ok(sqlx::query_as::<_, Group>(GET_USER_GROUPS)
        .bind(user_id)
        .fetch_all(pool)
        .await?)
}

pub async fn update_group(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
) -> anyhow::Result<Group> {
    Ok(sqlx::query_as::<_, Group>(UPDATE)
        .bind(id)
        .bind(name)
        .bind(description)
        .fetch_one(pool)
        .await?)
}