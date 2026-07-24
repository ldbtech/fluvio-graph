//! invites.rs table queries - pure data access no business logic. 
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::db::queries::invites::{Invite, CREATE, GET_BY_TOKEN, ACCEPT, GET_GROUP_INVITES};

pub async fn create_invite(
    pool: &PgPool,
    group_id: Uuid,
    invited_by: Uuid,
    role: &str,
    email: Option<&str>,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<Invite> {
    let token = Uuid::new_v4().to_string();

    Ok(sqlx::query_as::<_, Invite>(CREATE)
        .bind(group_id)
        .bind(invited_by)
        .bind(token)
        .bind(role)
        .bind(email)
        .bind(expires_at)
        .fetch_one(pool)
        .await?)
}

pub async fn get_invite_by_token(
    pool: &PgPool,
    token: &str,
) -> anyhow::Result<Option<Invite>> {
    Ok(sqlx::query_as::<_, Invite>(GET_BY_TOKEN)
        .bind(token)
        .fetch_optional(pool)
        .await?)
}

pub async fn accept_invite(
    pool: &PgPool,
    token: &str,
    accepted_by: Uuid,
) -> anyhow::Result<Invite> {
    Ok(sqlx::query_as::<_, Invite>(ACCEPT)
        .bind(token)
        .bind(accepted_by)
        .fetch_one(pool)
        .await?)
}

pub async fn get_group_invites(
    pool: &PgPool,
    group_id: Uuid,
) -> anyhow::Result<Vec<Invite>> {
    Ok(sqlx::query_as::<_, Invite>(GET_GROUP_INVITES)
        .bind(group_id)
        .fetch_all(pool)
        .await?)
}