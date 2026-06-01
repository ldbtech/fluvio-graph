//! Database operations for the fluvio_company database.
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExecutionLog {
    pub id:                     Uuid,
    pub company_id:             Uuid,
    pub initiated_by_user_id:   Uuid,
    pub initiated_by_twin_id:   Option<Uuid>,
    pub agent_name:             String,
    pub message:                String,
    pub log_level:              String,
    pub timestamp:              DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActionAuthorization {
    pub id:                     Uuid,
    pub company_id:             Uuid,
    pub action_type:            String,
    pub description:            String,
    pub severity:               String,
    pub initiated_by_user_id:   Uuid,
    pub status:                 String,
    pub authorized_by_user_id:  Option<Uuid>,
    pub notes:                  Option<String>,
    pub created_at:             DateTime<Utc>,
    pub resolved_at:            Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DocumentReconciliation {
    pub id:              Uuid,
    pub company_id:      Uuid,
    pub title:           String,
    pub description:     String,
    pub source_a:        String,
    pub source_b:        String,
    pub resolved_to:     String,
    pub time_ago:        String,
    pub created_at:      DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PipelineRun {
    pub id:              Uuid,
    pub company_id:      Uuid,
    pub name:            String,
    pub agent_name:      String,
    pub status:          String,
    pub progress:        i32,
    pub detail:          Option<String>,
    pub started_at:      DateTime<Utc>,
    pub updated_at:      DateTime<Utc>,
}

pub async fn get_execution_logs(
    pool:       &PgPool,
    company_id: Uuid,
    limit:      i64,
) -> anyhow::Result<Vec<ExecutionLog>> {
    Ok(sqlx::query_as::<_, ExecutionLog>(
        "SELECT id, company_id, initiated_by_user_id, initiated_by_twin_id, agent_name, message, log_level, timestamp
         FROM execution_logs
         WHERE company_id = $1
         ORDER BY timestamp DESC
         LIMIT $2"
    )
    .bind(company_id)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

pub async fn create_execution_log(
    pool:                 &PgPool,
    company_id:           Uuid,
    initiated_by_user_id: Uuid,
    initiated_by_twin_id: Option<Uuid>,
    agent_name:           &str,
    message:              &str,
    log_level:            &str,
) -> anyhow::Result<ExecutionLog> {
    Ok(sqlx::query_as::<_, ExecutionLog>(
        "INSERT INTO execution_logs (company_id, initiated_by_user_id, initiated_by_twin_id, agent_name, message, log_level)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, company_id, initiated_by_user_id, initiated_by_twin_id, agent_name, message, log_level, timestamp"
    )
    .bind(company_id)
    .bind(initiated_by_user_id)
    .bind(initiated_by_twin_id)
    .bind(agent_name)
    .bind(message)
    .bind(log_level)
    .fetch_one(pool)
    .await?)
}

pub async fn get_action_authorizations(
    pool:       &PgPool,
    company_id: Uuid,
    status:     Option<&str>,
) -> anyhow::Result<Vec<ActionAuthorization>> {
    if let Some(status_val) = status {
        Ok(sqlx::query_as::<_, ActionAuthorization>(
            "SELECT id, company_id, action_type, description, severity, initiated_by_user_id, status, authorized_by_user_id, notes, created_at, resolved_at
             FROM action_authorizations
             WHERE company_id = $1 AND status = $2
             ORDER BY created_at DESC"
        )
        .bind(company_id)
        .bind(status_val)
        .fetch_all(pool)
        .await?)
    } else {
        Ok(sqlx::query_as::<_, ActionAuthorization>(
            "SELECT id, company_id, action_type, description, severity, initiated_by_user_id, status, authorized_by_user_id, notes, created_at, resolved_at
             FROM action_authorizations
             WHERE company_id = $1
             ORDER BY created_at DESC"
        )
        .bind(company_id)
        .fetch_all(pool)
        .await?)
    }
}

pub async fn resolve_action_authorization(
    pool:                  &PgPool,
    id:                    Uuid,
    status:                &str,
    authorized_by_user_id: Option<Uuid>,
    notes:                 Option<&str>,
) -> anyhow::Result<ActionAuthorization> {
    Ok(sqlx::query_as::<_, ActionAuthorization>(
        "UPDATE action_authorizations
         SET status = $1, authorized_by_user_id = $2, notes = $3, resolved_at = now()
         WHERE id = $4
         RETURNING id, company_id, action_type, description, severity, initiated_by_user_id, status, authorized_by_user_id, notes, created_at, resolved_at"
    )
    .bind(status)
    .bind(authorized_by_user_id)
    .bind(notes)
    .bind(id)
    .fetch_one(pool)
    .await?)
}

pub async fn get_document_reconciliations(
    pool:       &PgPool,
    company_id: Uuid,
) -> anyhow::Result<Vec<DocumentReconciliation>> {
    Ok(sqlx::query_as::<_, DocumentReconciliation>(
        "SELECT id, company_id, title, description, source_a, source_b, resolved_to, time_ago, created_at
         FROM document_reconciliations
         WHERE company_id = $1
         ORDER BY created_at DESC"
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_pipeline_runs(
    pool:       &PgPool,
    company_id: Uuid,
) -> anyhow::Result<Vec<PipelineRun>> {
    Ok(sqlx::query_as::<_, PipelineRun>(
        "SELECT id, company_id, name, agent_name, status, progress, detail, started_at, updated_at
         FROM pipeline_runs
         WHERE company_id = $1
         ORDER BY updated_at DESC"
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?)
}

pub async fn delete_company_data(pool: &PgPool, company_id: Uuid) -> anyhow::Result<bool> {
    sqlx::query("DELETE FROM execution_logs WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM action_authorizations WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM document_reconciliations WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM pipeline_runs WHERE company_id = $1")
        .bind(company_id)
        .execute(pool)
        .await?;
    Ok(true)
}

