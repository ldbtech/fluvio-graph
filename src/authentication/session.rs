//! auth/session.rs
//!
//! Axum extractor that validates a Bearer token from the
//! Authorization header and returns the authenticated user.
//!
//! Usage in route handlers:
//!   async fn my_handler(
//!     AuthUser(user): AuthUser,  // 401 if not authenticated
//!     State(state): State<AppState>,
//!   ) -> ...
//!
//!   async fn my_optional_handler(
//!     maybe: Option<AuthUser>,   // None if not authenticated
//!   ) -> ...

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};

use sqlx::PgPool;

use crate::app_state::AppState;
use crate::database::auth::get_session_by_token;
use crate::database::users::{User, get_user_by_id};

// ── AuthUser extractor ────────────────────────────────────────────────────────

/// Authenticated user — extracted from Bearer token in Authorization header.
/// Returns 401 if token is missing, invalid, or expired.
pub struct AuthUser(pub User);

struct AuthUserParts {
    token: Option<String>,
    pool: PgPool,
}

#[async_trait]
trait IntoAuthUser {
    async fn into_auth_user(self) -> Result<AuthUser, (StatusCode, String)>;
}

#[async_trait]
impl IntoAuthUser for AuthUserParts {
    async fn into_auth_user(self) -> Result<AuthUser, (StatusCode, String)> {
        let token = self
            .token
            .ok_or((StatusCode::UNAUTHORIZED, "Authorization header required".into()))?;

        let session = get_session_by_token(&self.pool, &token)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid or expired session".into()))?;

        let user = get_user_by_id(&self.pool, session.user_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found".into()))?;

        Ok(AuthUser(user))
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, String);

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        AuthUserParts {
            token: extract_bearer(parts),
            pool: state.pg_pool.clone(),
        }
        .into_auth_user()
    }
}

// ── OptionalAuthUser extractor ────────────────────────────────────────────────

/// Optional authenticated user.
/// Returns None if no token or invalid token — does not reject the request.
pub struct OptionalAuthUser(pub Option<User>);

struct OptionalAuthUserParts {
    token: Option<String>,
    pool: PgPool,
}

#[async_trait]
trait IntoOptionalAuthUser {
    async fn into_optional(self) -> Result<OptionalAuthUser, (StatusCode, String)>;
}

#[async_trait]
impl IntoOptionalAuthUser for OptionalAuthUserParts {
    async fn into_optional(self) -> Result<OptionalAuthUser, (StatusCode, String)> {
        let Some(token) = self.token else {
            return Ok(OptionalAuthUser(None));
        };

        let Ok(Some(session)) = get_session_by_token(&self.pool, &token).await else {
            return Ok(OptionalAuthUser(None));
        };

        let user = get_user_by_id(&self.pool, session.user_id)
            .await
            .ok()
            .flatten();

        Ok(OptionalAuthUser(user))
    }
}

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = (StatusCode, String);

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        OptionalAuthUserParts {
            token: extract_bearer(parts),
            pool: state.pg_pool.clone(),
        }
        .into_optional()
    }
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Extract Bearer token from Authorization header.
pub fn extract_bearer(parts: &Parts) -> Option<String> {
    extract_bearer_headers(&parts.headers)
}

/// Extract Bearer token from a header map (e.g. axum's extracted `HeaderMap`).
pub fn extract_bearer_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn extract_bearer_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
