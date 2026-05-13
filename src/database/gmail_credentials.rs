//! Per-user Gmail OAuth tokens (PostgreSQL persistence).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GmailCredentialRecord {
    pub access_token:  String,
    pub refresh_token: String,
    pub expires_at:    DateTime<Utc>,
}

pub async fn gmail_credentials_exist(pool: &PgPool, user_id: Uuid) -> anyhow::Result<bool> {
    let row = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT EXISTS(SELECT 1 FROM user_gmail_credentials WHERE user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("gmail_credentials_exist: {e}"))?;
    Ok(row.unwrap_or(false))
}

pub async fn get_gmail_credentials(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Option<GmailCredentialRecord>> {
    sqlx::query_as::<_, GmailCredentialRecord>(
        r#"
        SELECT access_token, refresh_token, expires_at
        FROM user_gmail_credentials
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_gmail_credentials: {e}"))
}

pub async fn upsert_gmail_credentials(
    pool:    &PgPool,
    user_id: Uuid,
    access_token:  &str,
    refresh_token: &str,
    expires_at_unix: i64,
) -> anyhow::Result<()> {
    let expires_at = DateTime::<Utc>::from_timestamp(expires_at_unix, 0).unwrap_or_else(Utc::now);

    sqlx::query(
        r#"
        INSERT INTO user_gmail_credentials (user_id, access_token, refresh_token, expires_at, updated_at)
        VALUES ($1, $2, $3, $4, now())
        ON CONFLICT (user_id)
        DO UPDATE SET
            access_token  = EXCLUDED.access_token,
            refresh_token = EXCLUDED.refresh_token,
            expires_at    = EXCLUDED.expires_at,
            updated_at    = now()
        "#,
    )
    .bind(user_id)
    .bind(access_token)
    .bind(refresh_token)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("upsert_gmail_credentials: {e}"))?;

    Ok(())
}

pub async fn delete_gmail_credentials(pool: &PgPool, user_id: Uuid) -> anyhow::Result<bool> {
    let r = sqlx::query("DELETE FROM user_gmail_credentials WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("delete_gmail_credentials: {e}"))?;
    Ok(r.rows_affected() > 0)
}
