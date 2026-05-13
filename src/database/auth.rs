//! database/auth.rs
//!
//! Auth codes (OTP) and sessions CRUD.
//! Pure SQL — no HTTP, no business logic.

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── AuthCode ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthCode {
    pub id:         Uuid,
    pub email:      String,
    pub code:       String,
    pub expires_at: DateTime<Utc>,
    pub used:       bool,
    pub created_at: DateTime<Utc>,
}

/// Generate and store a 6-digit OTP code for an email.
/// Invalidates any previous unused codes for this email.
pub async fn create_auth_code(
    pool:  &PgPool,
    email: &str,
) -> anyhow::Result<String> {
    // Generate 6-digit code
    let code = format!("{:06}", rand_code());

    // Invalidate old codes for this email
    sqlx::query!(
        "UPDATE auth_codes SET used = true WHERE email = $1 AND used = false",
        email
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("invalidate old codes: {e}"))?;

    // Insert new code — expires in 10 minutes
    sqlx::query!(
        r#"
        INSERT INTO auth_codes (email, code, expires_at)
        VALUES ($1, $2, now() + interval '10 minutes')
        "#,
        email,
        code,
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create_auth_code: {e}"))?;

    Ok(code)
}

/// Verify a code for an email.
/// Returns true and marks code as used if valid.
/// Returns false if code is wrong, expired, or already used.
pub async fn verify_auth_code(
    pool:  &PgPool,
    email: &str,
    code:  &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"
        UPDATE auth_codes
        SET used = true
        WHERE email = $1
          AND code  = $2
          AND used  = false
          AND expires_at > now()
        RETURNING id
        "#,
        email,
        code,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("verify_auth_code: {e}"))?;

    Ok(result.is_some())
}

// ── Session ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id:         Uuid,
    pub user_id:    Uuid,
    pub token:      String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Create a new session for a user.
/// Returns the session token to be stored in the frontend.
pub async fn create_session(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Session> {
    let token = generate_token();

    let session = sqlx::query_as!(
        Session,
        r#"
        INSERT INTO sessions (user_id, token, expires_at)
        VALUES ($1, $2, now() + interval '30 days')
        RETURNING id, user_id, token, expires_at as "expires_at!", created_at as "created_at!"
        "#,
        user_id,
        token,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("create_session: {e}"))?;

    tracing::info!("[Auth] Session created for user {user_id}");
    Ok(session)
}

/// Look up a session by token.
/// Returns None if token not found or expired.
pub async fn get_session_by_token(
    pool:  &PgPool,
    token: &str,
) -> anyhow::Result<Option<Session>> {
    let session = sqlx::query_as!(
        Session,
        r#"
        SELECT id, user_id, token,
               expires_at as "expires_at!",
               created_at as "created_at!"
        FROM sessions
        WHERE token = $1 AND expires_at > now()
        "#,
        token,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_session_by_token: {e}"))?;

    Ok(session)
}

/// Delete a session — logout.
pub async fn delete_session(
    pool:  &PgPool,
    token: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE token = $1",
        token
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("delete_session: {e}"))?;

    Ok(result.rows_affected() > 0)
}

/// Delete all sessions for a user — logout everywhere.
pub async fn delete_all_sessions(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM sessions WHERE user_id = $1",
        user_id
    )
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("delete_all_sessions: {e}"))?;

    Ok(result.rows_affected())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Generate a cryptographically random 6-digit code.
fn rand_code() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    // Mix with a UUID for better randomness
    let mix = Uuid::new_v4().as_u128() as u64;
    ((nanos as u64 ^ mix) % 900_000 + 100_000) as u32
}

/// Generate a secure session token (UUID v4 — 128 bits of entropy).
fn generate_token() -> String {
    Uuid::new_v4().to_string()
}