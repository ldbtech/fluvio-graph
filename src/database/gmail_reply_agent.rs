//! Gmail reply agent settings + idempotency (which messages already handled).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Which knowledge sources may be stitched into reply prompts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GmailAgentContextSources {
    /// Signed-in Postgres profile (name, email, phone).
    #[serde(default = "default_true")]
    pub account_profile: bool,
    /// PDF + video uploads in Surreal (`source` pdf / video).
    #[serde(default = "default_true")]
    pub uploads: bool,
    /// Current codebase / GitHub ingest (`source` codebase, Codebase domain).
    #[serde(default = "default_true")]
    pub github_codebase: bool,
    #[serde(default = "default_true")]
    pub ingested_email: bool,
    #[serde(default = "default_true")]
    pub twin_notes: bool,
    /// Connected people (PostgreSQL roster + optional Surreal snippets per zone).
    #[serde(default = "default_true")]
    pub network_connections: bool,
}

fn default_true() -> bool {
    true
}

impl GmailAgentContextSources {
    pub fn normalize(self) -> Self {
        Self {
            account_profile:       self.account_profile,
            uploads:               self.uploads,
            github_codebase:       self.github_codebase,
            ingested_email:        self.ingested_email,
            twin_notes:            self.twin_notes,
            network_connections:   self.network_connections,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum GmailAgentSendMode {
    AlwaysReview,
    AutoWhenConfident,
}

impl GmailAgentSendMode {
    pub fn as_db(&self) -> &'static str {
        match self {
            GmailAgentSendMode::AlwaysReview => "always_review",
            GmailAgentSendMode::AutoWhenConfident => "auto_when_confident",
        }
    }

    pub fn from_db(s: &str) -> Self {
        match s {
            "auto_when_confident" => GmailAgentSendMode::AutoWhenConfident,
            _ => GmailAgentSendMode::AlwaysReview,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GmailReplyAgentRecord {
    pub enabled:             bool,
    pub auto_poll_enabled:   bool,
    pub send_mode:           GmailAgentSendMode,
    pub context_sources:     GmailAgentContextSources,
    pub updated_at:          DateTime<Utc>,
}

pub async fn ensure_reply_agent_row(pool: &PgPool, user_id: Uuid) -> anyhow::Result<()> {
    let default_ctx = serde_json::to_value(&GmailAgentContextSources::default())?;
    sqlx::query(r#"INSERT INTO user_gmail_reply_agent (user_id, context_sources)
           VALUES ($1, $2)
           ON CONFLICT (user_id) DO NOTHING"#)
    .bind(user_id)
    .bind(default_ctx)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("ensure_reply_agent_row: {e}"))?;
    Ok(())
}

pub async fn get_reply_agent_settings(
    pool:    &PgPool,
    user_id: Uuid,
) -> anyhow::Result<GmailReplyAgentRecord> {
    ensure_reply_agent_row(pool, user_id).await?;
    let row = sqlx::query(
        r#"
        SELECT enabled, COALESCE(auto_poll_enabled, false) AS auto_poll_enabled,
               send_mode, context_sources, updated_at
        FROM user_gmail_reply_agent
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_reply_agent_settings: {e}"))?;

    let j = row.try_get::<serde_json::Value, _>("context_sources").unwrap_or(serde_json::json!({}));
    let ctx: GmailAgentContextSources = serde_json::from_value(j).unwrap_or_default();

    Ok(GmailReplyAgentRecord {
        enabled:             row.get::<bool, _>("enabled"),
        auto_poll_enabled:   row.get::<bool, _>("auto_poll_enabled"),
        send_mode:           GmailAgentSendMode::from_db(row.get::<String, _>("send_mode").as_str()),
        context_sources:     ctx.normalize(),
        updated_at:          row.get("updated_at"),
    })
}

/// Users who asked for background replies and have Gmail tokens stored.
/// Every user with stored Gmail OAuth receives automatic inbox passes (server tick).
pub async fn list_user_ids_with_gmail_credentials(pool: &PgPool) -> anyhow::Result<Vec<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT user_id
        FROM user_gmail_credentials
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_user_ids_with_gmail_credentials: {e}"))
}

pub async fn put_reply_agent_settings(
    pool:      &PgPool,
    user_id:   Uuid,
    send_mode: &GmailAgentSendMode,
    ctx:       GmailAgentContextSources,
) -> anyhow::Result<GmailReplyAgentRecord> {
    ensure_reply_agent_row(pool, user_id).await?;
    let ctx_norm = ctx.normalize();
    let json = serde_json::to_value(&ctx_norm)?;
    sqlx::query(
        r#"
        UPDATE user_gmail_reply_agent
        SET enabled = true,
            auto_poll_enabled = true,
            send_mode = $2,
            context_sources = $3,
            updated_at = now()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(send_mode.as_db())
    .bind(json)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("put_reply_agent_settings: {e}"))?;

    get_reply_agent_settings(pool, user_id).await
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReviewDraftRow {
    pub gmail_message_id: String,
    pub thread_id:        Option<String>,
    pub subject_hint:     Option<String>,
    pub reply_proposal:   Option<String>,
    pub detail:           Option<String>,
    pub processed_at:     DateTime<Utc>,
}

/// Saved proposal rows (`draft_only`) for inbox review UX.
pub async fn list_agent_review_drafts(pool: &PgPool, user_id: Uuid, limit: i64) -> anyhow::Result<Vec<AgentReviewDraftRow>> {
    let lim = limit.clamp(1, 200);
    let rows = sqlx::query(
        r#"
        SELECT gmail_message_id, thread_id, subject_hint, reply_proposal, detail, processed_at
        FROM user_gmail_agent_processed
        WHERE user_id = $1 AND outcome = 'draft_only'
        ORDER BY processed_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(lim)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_agent_review_drafts: {e}"))?;

    Ok(rows
        .into_iter()
        .map(|row| AgentReviewDraftRow {
            gmail_message_id: row.get("gmail_message_id"),
            thread_id: row.get("thread_id"),
            subject_hint: row.get("subject_hint"),
            reply_proposal: row.get("reply_proposal"),
            detail: row.get("detail"),
            processed_at: row.get("processed_at"),
        })
        .collect())
}

pub async fn agent_message_processed(pool: &PgPool, user_id: Uuid, gmail_id: &str) -> anyhow::Result<bool> {
    let r = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM user_gmail_agent_processed
            WHERE user_id = $1 AND gmail_message_id = $2
        )
        "#,
    )
    .bind(user_id)
    .bind(gmail_id)
    .fetch_one(pool)
    .await
    .map_err(|e| anyhow::anyhow!("agent_message_processed: {e}"))?;
    Ok(r)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessOutcomeDb {
    Skipped,
    DraftOnly,
    Sent,
    Error,
}

impl ProcessOutcomeDb {
    fn as_str(&self) -> &'static str {
        match self {
            ProcessOutcomeDb::Skipped => "skipped",
            ProcessOutcomeDb::DraftOnly => "draft_only",
            ProcessOutcomeDb::Sent => "sent",
            ProcessOutcomeDb::Error => "error",
        }
    }
}

pub async fn mark_agent_processed(
    pool:           &PgPool,
    user_id:        Uuid,
    gmail_id:       &str,
    outcome:        ProcessOutcomeDb,
    detail:         Option<&str>,
    reply_proposal: Option<&str>,
    thread_id:      Option<&str>,
    subject_hint:   Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_gmail_agent_processed (
            user_id, gmail_message_id, outcome, detail,
            reply_proposal, thread_id, subject_hint
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (user_id, gmail_message_id)
        DO UPDATE SET
            outcome = EXCLUDED.outcome,
            detail = EXCLUDED.detail,
            reply_proposal = EXCLUDED.reply_proposal,
            thread_id = EXCLUDED.thread_id,
            subject_hint = EXCLUDED.subject_hint,
            processed_at = now()
        "#,
    )
    .bind(user_id)
    .bind(gmail_id)
    .bind(outcome.as_str())
    .bind(detail)
    .bind(reply_proposal)
    .bind(thread_id)
    .bind(subject_hint)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("mark_agent_processed: {e}"))?;
    Ok(())
}
