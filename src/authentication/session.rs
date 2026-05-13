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
//! **Global:** `require_logged_in_session` rejects requests without a valid `Authorization: Bearer`
//! session token, except OAuth/bootstrap paths (see `route_allows_anonymous`).
//!
//! For multipart uploads (`multipart/form-data`), **also** apply `multipart_upload_must_be_logged_in`
//! on the ingest router: it re-validates the session and sets internal `x-fluvio-upload-user-id`
//! (stripped from client requests). Handlers take `Request`, read that header via
//! `upload_user_id_from_headers`, then call `Multipart::from_request` so the handler future stays
//! `Send`.
//!
//!   async fn my_optional_handler(
//!     maybe: Option<AuthUser>,   // None if not authenticated
//!   ) -> ...

use async_trait::async_trait;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{
        header::{HeaderName, HeaderValue},
        request::Parts,
        HeaderMap, Method,
        StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};

use sqlx::PgPool;
use uuid::Uuid;

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

/// Routes registered **under** `require_logged_in_session` but that must bypass the Bearer check.
/// OTP endpoints live on a separate public router (`public_auth_routes`) and never hit this.
pub fn route_allows_anonymous(method: &Method, path: &str) -> bool {
    if *method == Method::OPTIONS {
        return true;
    }
    let p = path.trim_end_matches('/');
    match *method {
        // Logout: client may have cleared the token locally before calling DELETE.
        Method::DELETE if p == "/twin/auth/session" => true,
        // Google redirects the browser to these URLs without an Authorization header.
        // Browser hits this after Google redirects; no Bearer on the navigation request.
        Method::GET if p == "/connect/gmail/callback" => true,
        _ => false,
    }
}

/// Rejects the request unless `OptionalAuthUser` resolves to a real user.
/// Apply to the main API router; mount email OTP `request` / `verify` (and a few OAuth paths) **outside** this layer.
pub async fn require_logged_in_session(
    State(app_state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();

    if route_allows_anonymous(&method, &path) {
        return next.run(req).await;
    }

    let (mut parts, body) = req.into_parts();

    let user = match OptionalAuthUser::from_request_parts(&mut parts, &app_state).await {
        Ok(OptionalAuthUser(u)) => u,
        Err(e) => return e.into_response(),
    };

    if user.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            "sign in required — send Authorization: Bearer <session token> from POST /twin/auth/verify",
        )
            .into_response();
    }

    next.run(Request::from_parts(parts, body)).await
}

static HDR_UPLOAD_USER_ID: HeaderName = HeaderName::from_static("x-fluvio-upload-user-id");

fn trust_upload_user_id_header(uid: Uuid) -> Result<HeaderValue, Response> {
    HeaderValue::from_str(&uid.to_string()).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid internal upload header",
        )
            .into_response()
    })
}

/// Validates `Authorization: Bearer`, then stamps `x-fluvio-upload-user-id` (internal-only;
/// stripped from incoming requests so clients cannot spoof it).
pub async fn multipart_upload_must_be_logged_in(
    State(app_state): State<AppState>,
    req:              Request,
    next:             Next,
) -> Response {
    let (mut parts, body) = req.into_parts();

    let maybe = match OptionalAuthUser::from_request_parts(&mut parts, &app_state).await {
        Ok(v)  => v,
        Err(e) => return e.into_response(),
    };

    let Some(user) = maybe.0 else {
        return (
            StatusCode::UNAUTHORIZED,
            "sign in required — send Authorization: Bearer <session token> from POST /twin/auth/verify",
        )
            .into_response();
    };

    parts.headers.remove(&HDR_UPLOAD_USER_ID);
    let hv = match trust_upload_user_id_header(user.id) {
        Ok(v)  => v,
        Err(r) => return r,
    };
    parts.headers.insert(HDR_UPLOAD_USER_ID.clone(), hv);

    next.run(Request::from_parts(parts, body)).await
}

pub fn upload_user_id_from_headers(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get(&HDR_UPLOAD_USER_ID)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
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
