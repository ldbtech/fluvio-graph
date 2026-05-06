//! database/users.rs
//!
//! User CRUD — clean database operations only.
//! No business logic here — just SQL.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── User struct ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id:         Uuid,
    pub name:       String,
    pub email:      Option<String>,
    pub phone:      Option<String>,
    pub graph_id:   Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name:  String,
    pub email: Option<String>,
    pub phone: Option<String>,
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

/// Insert a new user. Returns the created user with generated id + graph_id.
pub async fn create_user(pool: &PgPool, input: &CreateUser) -> anyhow::Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (name, email, phone)
        VALUES ($1, $2, $3)
        RETURNING id, name, email, phone, graph_id, created_at
        "#,
        input.name,
        input.email,
        input.phone,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create_user failed: {e}"))?;

    tracing::info!("[DB] Created user: {} ({})", user.name, user.id);
    Ok(user)
}

/// Fetch a user by their UUID.
pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, name, email, phone, graph_id, created_at
        FROM users WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_user_by_id failed: {e}"))?;

    Ok(user)
}

/// Fetch a user by their graph_id.
/// Used when traversing from a graph node back to the user record.
pub async fn get_user_by_graph_id(pool: &PgPool, graph_id: Uuid) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, name, email, phone, graph_id, created_at
        FROM users WHERE graph_id = $1
        "#,
        graph_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_user_by_graph_id failed: {e}"))?;

    Ok(user)
}

/// Fetch a user by email.
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, name, email, phone, graph_id, created_at
        FROM users WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_user_by_email failed: {e}"))?;

    Ok(user)
}

/// Update user name, email, or phone.
pub async fn update_user(
    pool:  &PgPool,
    id:    Uuid,
    name:  Option<&str>,
    email: Option<&str>,
    phone: Option<&str>,
) -> anyhow::Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET
            name  = COALESCE($2, name),
            email = COALESCE($3, email),
            phone = COALESCE($4, phone)
        WHERE id = $1
        RETURNING id, name, email, phone, graph_id, created_at
        "#,
        id,
        name,
        email,
        phone,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("update_user failed: {e}"))?;

    tracing::info!("[DB] Updated user: {}", id);
    Ok(user)
}

/// Delete a user by id. Cascades to cards and connections.
pub async fn delete_user(pool: &PgPool, id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM users WHERE id = $1",
        id
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("delete_user failed: {e}"))?;

    Ok(result.rows_affected() > 0)
}

/// List all users — admin use only.
pub async fn list_users(pool: &PgPool) -> anyhow::Result<Vec<User>> {
    let users = sqlx::query_as!(
        User,
        r#"
        SELECT id, name, email, phone, graph_id, created_at
        FROM users ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_users failed: {e}"))?;

    Ok(users)
}