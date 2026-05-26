//! teams.rs table queries - pure data access no business logic.
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Team {
    pub id:          Uuid,
    pub company_id:  Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TeamMember {
    pub id:        Uuid,
    pub team_id:   Uuid,
    pub user_id:   Uuid,
    pub role:      String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TeamWorkflow {
    pub id:          Uuid,
    pub team_id:     Uuid,
    pub name:        String,
    pub description: Option<String>,
    pub steps:       Value, // JSONB
    pub created_by:  Uuid,
    pub created_at:  DateTime<Utc>,
    pub updated_at:  DateTime<Utc>,
}

pub async fn create_team(
    pool:        &PgPool,
    company_id:  Uuid,
    name:        &str,
    description: Option<&str>,
) -> anyhow::Result<Team> {
    Ok(sqlx::query_as::<_, Team>(
        "INSERT INTO teams (company_id, name, description)
         VALUES ($1, $2, $3)
         RETURNING id, company_id, name, description, created_at, updated_at"
    )
    .bind(company_id)
    .bind(name)
    .bind(description)
    .fetch_one(pool)
    .await?)
}

pub async fn get_company_teams(pool: &PgPool, company_id: Uuid) -> anyhow::Result<Vec<Team>> {
    Ok(sqlx::query_as::<_, Team>(
        "SELECT id, company_id, name, description, created_at, updated_at
         FROM teams WHERE company_id = $1
         ORDER BY created_at DESC"
    )
    .bind(company_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_team(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Team>> {
    Ok(sqlx::query_as::<_, Team>(
        "SELECT id, company_id, name, description, created_at, updated_at
         FROM teams WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn add_team_member(
    pool:    &PgPool,
    team_id: Uuid,
    user_id: Uuid,
    role:    &str,
) -> anyhow::Result<TeamMember> {
    Ok(sqlx::query_as::<_, TeamMember>(
        "INSERT INTO team_members (team_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (team_id, user_id) DO UPDATE SET role = EXCLUDED.role
         RETURNING id, team_id, user_id, role, joined_at"
    )
    .bind(team_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await?)
}

pub async fn get_team_members(pool: &PgPool, team_id: Uuid) -> anyhow::Result<Vec<TeamMember>> {
    Ok(sqlx::query_as::<_, TeamMember>(
        "SELECT id, team_id, user_id, role, joined_at
         FROM team_members WHERE team_id = $1"
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?)
}

pub async fn create_team_workflow(
    pool:        &PgPool,
    team_id:     Uuid,
    name:        &str,
    description: Option<&str>,
    steps:       Value,
    created_by:  Uuid,
) -> anyhow::Result<TeamWorkflow> {
    Ok(sqlx::query_as::<_, TeamWorkflow>(
        "INSERT INTO team_workflows (team_id, name, description, steps, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, team_id, name, description, steps, created_by, created_at, updated_at"
    )
    .bind(team_id)
    .bind(name)
    .bind(description)
    .bind(steps)
    .bind(created_by)
    .fetch_one(pool)
    .await?)
}

pub async fn get_team_workflows(pool: &PgPool, team_id: Uuid) -> anyhow::Result<Vec<TeamWorkflow>> {
    Ok(sqlx::query_as::<_, TeamWorkflow>(
        "SELECT id, team_id, name, description, steps, created_by, created_at, updated_at
         FROM team_workflows WHERE team_id = $1
         ORDER BY created_at DESC"
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?)
}
