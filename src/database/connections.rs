//! database/connections.rs
//!
//! Connection CRUD — who tapped whose card.
//! Frictionless: tap creates connection instantly.
//! Zone controls what the other person can query.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Connection struct ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Connection {
    pub id:       Uuid,
    pub user_a:   Uuid,
    pub user_b:   Uuid,
    pub zone:     i16,         // 1 = working on / projects, 2 = contact + CV
    pub tapped_at: DateTime<Utc>,
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

/// Create a connection between two users instantly on NFC tap.
/// If connection already exists — returns the existing one.
pub async fn create_connection(
    pool:   &PgPool,
    user_a: Uuid,
    user_b: Uuid,
    zone:   i16,
) -> anyhow::Result<Connection> {
    // Use ON CONFLICT to handle re-taps gracefully
    let conn = sqlx::query_as!(
        Connection,
        r#"
        INSERT INTO connections (user_a, user_b, zone)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_a, user_b) DO UPDATE
            SET zone = EXCLUDED.zone,
                tapped_at = now()
        RETURNING id, user_a, user_b, zone, tapped_at
        "#,
        user_a,
        user_b,
        zone,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create_connection failed: {e}"))?;

    tracing::info!("[DB] Connected: {} ↔ {} zone={}", user_a, user_b, zone);
    Ok(conn)
}

/// Get the connection between two specific users.
/// Returns None if not connected.
pub async fn get_connection(
    pool:   &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> anyhow::Result<Option<Connection>> {
    let conn = sqlx::query_as!(
        Connection,
        r#"
        SELECT id, user_a, user_b, zone, tapped_at
        FROM connections
        WHERE (user_a = $1 AND user_b = $2)
           OR (user_a = $2 AND user_b = $1)
        "#,
        user_a,
        user_b,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_connection failed: {e}"))?;

    Ok(conn)
}

/// Get all connections for a user — their full network.
pub async fn get_connections(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Vec<Connection>> {
    let conns = sqlx::query_as!(
        Connection,
        r#"
        SELECT id, user_a, user_b, zone, tapped_at
        FROM connections
        WHERE user_a = $1 OR user_b = $1
        ORDER BY tapped_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_connections failed: {e}"))?;

    Ok(conns)
}

/// Get all user_ids connected to a user — for network traversal.
pub async fn get_connected_user_ids(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Vec<(Uuid, i16)>> {
    let rows = sqlx::query!(
        r#"
        SELECT
            CASE WHEN user_a = $1 THEN user_b ELSE user_a END AS "other_user_id!: Uuid",
            zone
        FROM connections
        WHERE user_a = $1 OR user_b = $1
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_connected_user_ids failed: {e}"))?;

    Ok(rows.into_iter().map(|r| (r.other_user_id, r.zone)).collect())
}

/// Update the zone for an existing connection.
pub async fn update_zone(
    pool:   &PgPool,
    user_a: Uuid,
    user_b: Uuid,
    zone:   i16,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE connections SET zone = $3
        WHERE (user_a = $1 AND user_b = $2)
           OR (user_a = $2 AND user_b = $1)
        "#,
        user_a, user_b, zone
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("update_zone failed: {e}"))?;

    Ok(result.rows_affected() > 0)
}

/// Remove a connection — disconnect two users.
pub async fn delete_connection(
    pool:   &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"
        DELETE FROM connections
        WHERE (user_a = $1 AND user_b = $2)
           OR (user_a = $2 AND user_b = $1)
        "#,
        user_a, user_b
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("delete_connection failed: {e}"))?;

    Ok(result.rows_affected() > 0)
}

/// Check if two users are connected.
pub async fn are_connected(
    pool:   &PgPool,
    user_a: Uuid,
    user_b: Uuid,
) -> anyhow::Result<bool> {
    Ok(get_connection(pool, user_a, user_b).await?.is_some())
}