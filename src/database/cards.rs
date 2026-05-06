//! database/cards.rs
//!
//! Card CRUD — NFC, BLE, Apple Pay cards.
//! Each card maps to one user via card_id.
//! When a card is tapped, card_id → user lookup happens here.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Card struct ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Card {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub card_type:  String,   // "nfc" | "ble" | "apple_pay"
    pub created_at: DateTime<Utc>,
}

// ── CRUD ──────────────────────────────────────────────────────────────────────

/// Create a new card for a user.
pub async fn create_card(
    pool:      &PgPool,
    user_id:   Uuid,
    card_type: &str,
) -> anyhow::Result<Card> {
    let card = sqlx::query_as!(
        Card,
        r#"
        INSERT INTO cards (user_id, card_type)
        VALUES ($1, $2)
        RETURNING id, user_id, card_type, created_at
        "#,
        user_id,
        card_type,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create_card failed: {e}"))?;

    tracing::info!("[DB] Created {} card: {} for user {}", card_type, card.id, user_id);
    Ok(card)
}

/// Look up a card by its id.
/// This is the hot path — called every time an NFC card is tapped.
pub async fn get_card_by_id(pool: &PgPool, card_id: Uuid) -> anyhow::Result<Option<Card>> {
    let card = sqlx::query_as!(
        Card,
        r#"
        SELECT id, user_id, card_type, created_at
        FROM cards WHERE id = $1
        "#,
        card_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_card_by_id failed: {e}"))?;

    Ok(card)
}

/// Get all cards belonging to a user.
pub async fn get_cards_by_user(pool: &PgPool, user_id: Uuid) -> anyhow::Result<Vec<Card>> {
    let cards = sqlx::query_as!(
        Card,
        r#"
        SELECT id, user_id, card_type, created_at
        FROM cards WHERE user_id = $1
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_cards_by_user failed: {e}"))?;

    Ok(cards)
}

/// Delete a card by id.
pub async fn delete_card(pool: &PgPool, card_id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM cards WHERE id = $1",
        card_id
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("delete_card failed: {e}"))?;

    Ok(result.rows_affected() > 0)
}