//! users.rs table queries - pure data access no business logic. 
use sqlx::PgPool;
use uuid::Uuid;
use crate::db::queries::users::{User, GET_BY_ID, GET_BY_FIREBASE_UID, UPSERT, UPDATE, GET_BY_EMAIL};

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    Ok(sqlx::query_as::<_, User>(GET_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> anyhow::Result<Option<User>> {
    Ok(sqlx::query_as::<_, User>(GET_BY_EMAIL)
        .bind(email)
        .fetch_optional(pool)
        .await?)
}


pub async fn get_user_by_firebase_uid(pool: &PgPool, uid: &str) -> anyhow::Result<Option<User>> {
    Ok(sqlx::query_as::<_, User>(GET_BY_FIREBASE_UID)
        .bind(uid)
        .fetch_optional(pool)
        .await?)
}

pub async fn create_user(
    pool: &PgPool,
    firebase_uid: &str,
    email: Option<&str>,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> anyhow::Result<User> {
    Ok(sqlx::query_as::<_, User>(UPSERT)
        .bind(firebase_uid)
        .bind(email)
        .bind(display_name)
        .bind(avatar_url)
        .fetch_one(pool)
        .await?)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    email: Option<&str>,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> anyhow::Result<User> {
    Ok(sqlx::query_as::<_, User>(UPDATE)
        .bind(id)
        .bind(email)
        .bind(display_name)
        .bind(avatar_url)
        .bind(None::<&str>)
        .bind(None::<Uuid>)
        .fetch_one(pool)
        .await?)
}

pub async fn update_company_email(
    pool: &PgPool,
    id: Uuid,
    company_email: Option<&str>,
) -> anyhow::Result<User> {
    Ok(sqlx::query_as::<_, User>(UPDATE)
        .bind(id)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(company_email)
        .bind(None::<Uuid>)
        .fetch_one(pool)
        .await?)
}

pub async fn update_company_id(
    pool: &PgPool,
    id: Uuid,
    company_id: Option<Uuid>,
) -> anyhow::Result<User> {
    Ok(sqlx::query_as::<_, User>(UPDATE)
        .bind(id)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .bind(company_id)
        .fetch_one(pool)
        .await?)
}