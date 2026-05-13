//! Focused senders (allow-list) and Gmail History cursor for incremental deltas.

use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

const MAX_FOCUS_SENDERS: usize = 48;

/// Trim, lowercase; full `a@b.c` or domain-only `@corp.com` (maps to *@corp.com in Gmail `q`).
pub fn normalize_focus_sender(raw: &str) -> Option<String> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() || s.len() > 254 {
        return None;
    }
    if s.starts_with('@') {
        let dom = s[1..].trim();
        if dom.is_empty()
            || dom.contains([' ', '\t', '"', '(', ')', ':'])
            || !dom.contains('.')
        {
            return None;
        }
        return Some(format!("@{dom}"));
    }
    if s.contains('@')
        && !s.chars().any(|c| matches!(c, ' ' | '\t' | '"' | '(' | ')' | ':' | '<' | '>'))
    {
        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Some(s);
        }
    }
    None
}

pub async fn list_focus_senders(pool: &PgPool, user_id: Uuid) -> anyhow::Result<Vec<String>> {
    let rows =
        sqlx::query_scalar::<_, String>("SELECT sender FROM user_gmail_focus_senders WHERE user_id = $1 ORDER BY sender")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .context("list_focus_senders")?;
    Ok(rows)
}

pub async fn replace_focus_senders(
    pool:    &PgPool,
    user_id: Uuid,
    raw:     &[String],
) -> anyhow::Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for r in raw.iter().take(MAX_FOCUS_SENDERS + 32) {
        if let Some(s) = normalize_focus_sender(r) {
            if !out.contains(&s) {
                out.push(s);
            }
        }
    }
    out.truncate(MAX_FOCUS_SENDERS);

    let mut tx = pool.begin().await.context("replace_focus_senders begin")?;
    sqlx::query("DELETE FROM user_gmail_focus_senders WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .context("replace_focus_senders delete")?;
    for sender in &out {
        sqlx::query(
            "INSERT INTO user_gmail_focus_senders (user_id, sender) VALUES ($1, $2) ON CONFLICT (user_id, sender) DO NOTHING",
        )
        .bind(user_id)
        .bind(sender)
        .execute(&mut *tx)
        .await
        .context("replace_focus_senders insert")?;
    }
    tx.commit().await.context("replace_focus_senders commit")?;
    Ok(out)
}

pub async fn get_history_cursor(pool: &PgPool, user_id: Uuid) -> anyhow::Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT history_id FROM user_gmail_history_cursor WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .context("get_history_cursor")
}

pub async fn set_history_cursor(pool: &PgPool, user_id: Uuid, history_id: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_gmail_history_cursor (user_id, history_id, updated_at)
        VALUES ($1, $2, now())
        ON CONFLICT (user_id) DO UPDATE SET
            history_id = EXCLUDED.history_id,
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(history_id)
    .execute(pool)
    .await
    .context("set_history_cursor")?;
    Ok(())
}
