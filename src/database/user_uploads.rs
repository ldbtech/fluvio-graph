//! Rows for dashboard “uploaded files” — PDFs, videos, and linked GitHub codebases ingested into Surreal-backed graph.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct UserUpload {
    pub id:            Uuid,
    pub user_id:       Uuid,
    pub kind:          String,
    pub file_name:     String,
    pub document_id:   Option<String>,
    pub graph_nodes:   Option<i32>,
    pub graph_edges:   Option<i32>,
    pub created_at:    DateTime<Utc>,
}

pub async fn insert_user_upload(
    pool:          &PgPool,
    user_id:       Uuid,
    kind:          &str,
    file_name:     &str,
    document_id:   Option<&str>,
    graph_nodes:   i32,
    graph_edges:   i32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO user_uploads (user_id, kind, file_name, document_id, graph_nodes, graph_edges)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(kind)
    .bind(file_name)
    .bind(document_id)
    .bind(graph_nodes)
    .bind(graph_edges)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("insert_user_upload: {e}"))?;
    Ok(())
}

pub async fn list_user_uploads_for_user_by_kind(
    pool:    &PgPool,
    user_id: Uuid,
    kind:    &str,
    limit:   i64,
) -> anyhow::Result<Vec<UserUpload>> {
    let rows = sqlx::query_as::<_, UserUpload>(
        r#"
        SELECT id, user_id, kind, file_name, document_id, graph_nodes, graph_edges, created_at
        FROM user_uploads
        WHERE user_id = $1 AND kind = $2
        ORDER BY created_at DESC
        LIMIT $3
        "#,
    )
    .bind(user_id)
    .bind(kind)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_user_uploads_for_user_by_kind: {e}"))?;
    Ok(rows)
}

pub async fn list_user_uploads_for_user(
    pool:    &PgPool,
    user_id: Uuid,
    limit:   i64,
) -> anyhow::Result<Vec<UserUpload>> {
    let rows = sqlx::query_as::<_, UserUpload>(
        r#"
        SELECT id, user_id, kind, file_name, document_id, graph_nodes, graph_edges, created_at
        FROM user_uploads
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow::anyhow!("list_user_uploads_for_user: {e}"))?;
    Ok(rows)
}

/// One upload row for this user, if it exists.
pub async fn get_user_upload_row(
    pool:      &PgPool,
    user_id:   Uuid,
    upload_id: Uuid,
) -> anyhow::Result<Option<UserUpload>> {
    let row = sqlx::query_as::<_, UserUpload>(
        r#"
        SELECT id, user_id, kind, file_name, document_id, graph_nodes, graph_edges, created_at
        FROM user_uploads
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(upload_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| anyhow::anyhow!("get_user_upload_row: {e}"))?;
    Ok(row)
}

/// Deletes one upload row owned by `user_id`. Returns rows affected (0 or 1).
pub async fn delete_user_upload_row(
    pool:      &PgPool,
    user_id:   Uuid,
    upload_id: Uuid,
) -> anyhow::Result<u64> {
    let r = sqlx::query(r#"DELETE FROM user_uploads WHERE id = $1 AND user_id = $2"#)
        .bind(upload_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("delete_user_upload_row: {e}"))?;
    Ok(r.rows_affected())
}

/// Deletes every library row for `kind` (e.g. replacing a linked codebase before a new ingest).
pub async fn delete_user_uploads_by_kind(
    pool:    &PgPool,
    user_id: Uuid,
    kind:    &str,
) -> anyhow::Result<u64> {
    let r = sqlx::query(r#"DELETE FROM user_uploads WHERE user_id = $1 AND kind = $2"#)
        .bind(user_id)
        .bind(kind)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("delete_user_uploads_by_kind: {e}"))?;
    Ok(r.rows_affected())
}
