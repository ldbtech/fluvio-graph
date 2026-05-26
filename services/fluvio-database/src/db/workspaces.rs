//! workspaces.rs table queries - pure data access no business logic.
use sqlx::PgPool;
use uuid::Uuid;
use crate::db::queries::workspaces::{
    Workspace, WorkspaceShare, WorkspaceShareWithUser,
    CREATE, GET_BY_ID, GET_USER_WORKSPACES, UPDATE, DELETE, SHARE, UNSHARE, GET_SHARES
};

pub async fn create_workspace(
    pool: &PgPool,
    owner_id: Uuid,
    name: &str,
    is_public: bool,
) -> anyhow::Result<Workspace> {
    Ok(sqlx::query_as::<_, Workspace>(CREATE)
        .bind(owner_id)
        .bind(name)
        .bind(is_public)
        .fetch_one(pool)
        .await?)
}

pub async fn get_workspace_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Workspace>> {
    Ok(sqlx::query_as::<_, Workspace>(GET_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_user_workspaces(pool: &PgPool, user_id: Uuid) -> anyhow::Result<Vec<Workspace>> {
    Ok(sqlx::query_as::<_, Workspace>(GET_USER_WORKSPACES)
        .bind(user_id)
        .fetch_all(pool)
        .await?)
}

pub async fn update_workspace(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    is_public: Option<bool>,
) -> anyhow::Result<Workspace> {
    Ok(sqlx::query_as::<_, Workspace>(UPDATE)
        .bind(id)
        .bind(name)
        .bind(is_public)
        .fetch_one(pool)
        .await?)
}

pub async fn delete_workspace(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query(DELETE)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn share_workspace(
    pool: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<WorkspaceShare> {
    Ok(sqlx::query_as::<_, WorkspaceShare>(SHARE)
        .bind(workspace_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?)
}

pub async fn unshare_workspace(
    pool: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<()> {
    sqlx::query(UNSHARE)
        .bind(workspace_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_workspace_shares(
    pool: &PgPool,
    workspace_id: Uuid,
) -> anyhow::Result<Vec<WorkspaceShareWithUser>> {
    Ok(sqlx::query_as::<_, WorkspaceShareWithUser>(GET_SHARES)
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
}

pub async fn verify_workspace_access(
    pool: &PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<()> {
    let ws = get_workspace_by_id(pool, workspace_id).await?;
    if let Some(w) = ws {
        if w.owner_id == user_id || w.is_public {
            return Ok(());
        }
    } else {
        anyhow::bail!("workspace not found");
    }

    let shared: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM workspace_shares WHERE workspace_id = $1 AND user_id = $2)"
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if shared {
        Ok(())
    } else {
        anyhow::bail!("access denied to workspace")
    }
}

