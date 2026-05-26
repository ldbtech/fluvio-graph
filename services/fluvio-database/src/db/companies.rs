//! companies.rs table queries - pure data access no business logic.
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Company {
    pub id:           Uuid,
    pub name:         String,
    pub website:      String,
    pub linkedin_url: String,
    pub twitter_url:  Option<String>,
    pub github_url:   Option<String>,
    pub created_by:   Uuid,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CompanyInvite {
    pub id:          Uuid,
    pub company_id:  Uuid,
    pub invited_by:  Uuid,
    pub email:       String,
    pub token:       String,
    pub role:        String,
    pub expires_at:  DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at:  DateTime<Utc>,
}

pub async fn create_company(
    pool:         &PgPool,
    name:         &str,
    website:      &str,
    linkedin_url: &str,
    twitter_url:  Option<&str>,
    github_url:   Option<&str>,
    created_by:   Uuid,
) -> anyhow::Result<Company> {
    Ok(sqlx::query_as::<_, Company>(
        "INSERT INTO companies (name, website, linkedin_url, twitter_url, github_url, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, name, website, linkedin_url, twitter_url, github_url, created_by, created_at, updated_at"
    )
    .bind(name)
    .bind(website)
    .bind(linkedin_url)
    .bind(twitter_url)
    .bind(github_url)
    .bind(created_by)
    .fetch_one(pool)
    .await?)
}

pub async fn get_company(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Company>> {
    Ok(sqlx::query_as::<_, Company>(
        "SELECT id, name, website, linkedin_url, twitter_url, github_url, created_by, created_at, updated_at
         FROM companies WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub async fn create_company_invite(
    pool:        &PgPool,
    company_id:  Uuid,
    invited_by:  Uuid,
    email:       &str,
    token:       &str,
    role:        &str,
    expires_at:  DateTime<Utc>,
) -> anyhow::Result<CompanyInvite> {
    Ok(sqlx::query_as::<_, CompanyInvite>(
        "INSERT INTO company_invites (company_id, invited_by, email, token, role, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, company_id, invited_by, email, token, role, expires_at, accepted_at, created_at"
    )
    .bind(company_id)
    .bind(invited_by)
    .bind(email)
    .bind(token)
    .bind(role)
    .bind(expires_at)
    .fetch_one(pool)
    .await?)
}

pub async fn get_company_invite_by_token(
    pool:  &PgPool,
    token: &str,
) -> anyhow::Result<Option<CompanyInvite>> {
    Ok(sqlx::query_as::<_, CompanyInvite>(
        "SELECT id, company_id, invited_by, email, token, role, expires_at, accepted_at, created_at
         FROM company_invites WHERE token = $1"
    )
    .bind(token)
    .fetch_optional(pool)
    .await?)
}

pub async fn get_pending_company_invites_by_email(
    pool:  &PgPool,
    email: &str,
) -> anyhow::Result<Vec<CompanyInvite>> {
    Ok(sqlx::query_as::<_, CompanyInvite>(
        "SELECT id, company_id, invited_by, email, token, role, expires_at, accepted_at, created_at
         FROM company_invites
         WHERE LOWER(email) = LOWER($1) AND accepted_at IS NULL AND expires_at > now()"
    )
    .bind(email)
    .fetch_all(pool)
    .await?)
}

pub async fn accept_company_invite(
    pool:      &PgPool,
    invite_id: Uuid,
) -> anyhow::Result<CompanyInvite> {
    Ok(sqlx::query_as::<_, CompanyInvite>(
        "UPDATE company_invites
         SET accepted_at = now()
         WHERE id = $1
         RETURNING id, company_id, invited_by, email, token, role, expires_at, accepted_at, created_at"
    )
    .bind(invite_id)
    .fetch_one(pool)
    .await?)
}
