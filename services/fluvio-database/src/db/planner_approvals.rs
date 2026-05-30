use sqlx::PgPool;
use uuid::Uuid;
use serde_json::Value;
use crate::db::queries::planner_approvals::{
    PlannerApproval, CREATE, GET_BY_WORKSPACE, REVIEW
};

pub async fn create_planner_approval(
    pool: &PgPool,
    workspace_id: Uuid,
    suggested_by: Uuid,
    change_type: &str,
    change_details: Value,
) -> anyhow::Result<PlannerApproval> {
    Ok(sqlx::query_as::<_, PlannerApproval>(CREATE)
        .bind(workspace_id)
        .bind(suggested_by)
        .bind(change_type)
        .bind(change_details)
        .fetch_one(pool)
        .await?)
}

pub async fn get_workspace_approvals(
    pool: &PgPool,
    workspace_id: Uuid,
) -> anyhow::Result<Vec<PlannerApproval>> {
    Ok(sqlx::query_as::<_, PlannerApproval>(GET_BY_WORKSPACE)
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
}

pub async fn review_planner_approval(
    pool: &PgPool,
    approval_id: Uuid,
    status: &str,
    review_note: Option<&str>,
) -> anyhow::Result<PlannerApproval> {
    Ok(sqlx::query_as::<_, PlannerApproval>(REVIEW)
        .bind(approval_id)
        .bind(status)
        .bind(review_note)
        .fetch_one(pool)
        .await?)
}
