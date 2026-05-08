//! routes/auth.rs
//!
//! Authentication endpoints:
//!   POST /twin/auth/request  — email → generate OTP → send email
//!   POST /twin/auth/verify   — email + code → session token
//!   DELETE /twin/auth/session — logout (delete session)
//!   GET  /twin/auth/me       — validate token → return user

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app_state::AppState;
use crate::authentication::{AuthUser, extract_bearer_headers, send_otp_email};
use crate::database::{
    auth::{create_auth_code, verify_auth_code, create_session, delete_session},
    users::{create_user, get_user_by_email, CreateUser},
};

// ── POST /twin/auth/request ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuthRequestBody {
    pub email: String,
    pub name:  Option<String>,  // required for new users
}

#[derive(Serialize)]
pub struct AuthRequestResponse {
    pub ok:        bool,
    pub email:     String,
    pub sent:      bool,        // true = email sent, false = demo mode
    /// Only present in demo mode (no RESEND_API_KEY set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code:      Option<String>,
    pub message:   String,
}

pub async fn post_auth_request(
    State(state): State<AppState>,
    Json(body):   Json<AuthRequestBody>,
) -> Result<Json<AuthRequestResponse>, (StatusCode, String)> {
    let email = body.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "valid email required".into()));
    }

    // Create user if they don't exist yet
    let existing = get_user_by_email(&state.pg_pool, &email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_none() {
        let name = body.name
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| email.split('@').next().unwrap_or("Friend").to_string());

        create_user(&state.pg_pool, &CreateUser {
            name,
            email: Some(email.clone()),
            phone: None,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Generate OTP code
    let code = create_auth_code(&state.pg_pool, &email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Send email (or demo mode)
    let resend_key = std::env::var("RESEND_API_KEY").ok();
    let sent = send_otp_email(&email, &code, resend_key.as_deref())
        .await
        .unwrap_or(false);

    let demo_code = if !sent { Some(code) } else { None };

    Ok(Json(AuthRequestResponse {
        ok:      true,
        email,
        sent,
        code:    demo_code,
        message: if sent {
            "Check your email for the login code".into()
        } else {
            "Demo mode — code returned in response".into()
        },
    }))
}

// ── POST /twin/auth/verify ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuthVerifyBody {
    pub email: String,
    pub code:  String,
}

#[derive(Serialize)]
pub struct AuthVerifyResponse {
    pub ok:       bool,
    pub token:    String,
    pub user_id:  Uuid,
    pub name:     String,
    pub graph_id: Option<Uuid>,
}

pub async fn post_auth_verify(
    State(state): State<AppState>,
    Json(body):   Json<AuthVerifyBody>,
) -> Result<Json<AuthVerifyResponse>, (StatusCode, String)> {
    let email = body.email.trim().to_lowercase();
    let code  = body.code.trim().to_string();

    if email.is_empty() || code.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "email and code required".into()));
    }

    // Verify OTP
    let valid = verify_auth_code(&state.pg_pool, &email, &code)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, "Invalid or expired code".into()));
    }

    // Get user
    let user = get_user_by_email(&state.pg_pool, &email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".into()))?;

    // Create session
    let session = create_session(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("[Auth] Verified: {} ({})", user.name, user.id);

    Ok(Json(AuthVerifyResponse {
        ok:       true,
        token:    session.token,
        user_id:  user.id,
        name:     user.name,
        graph_id: user.graph_id,
    }))
}

// ── DELETE /twin/auth/session ─────────────────────────────────────────────────

#[derive(Serialize)]
pub struct LogoutResponse {
    pub ok: bool,
}

pub async fn delete_auth_session(
    State(state): State<AppState>,
    headers:      axum::http::HeaderMap,
) -> Result<Json<LogoutResponse>, (StatusCode, String)> {
    let token = extract_bearer_headers(&headers);

    if let Some(token) = token {
        delete_session(&state.pg_pool, &token)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(LogoutResponse { ok: true }))
}

// ── GET /twin/auth/me ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct MeResponse {
    pub user_id:  Uuid,
    pub name:     String,
    pub email:    Option<String>,
    pub graph_id: Option<Uuid>,
}

pub async fn get_auth_me(
    AuthUser(user): AuthUser,
) -> Json<MeResponse> {
    Json(MeResponse {
        user_id:  user.id,
        name:     user.name,
        email:    user.email,
        graph_id: user.graph_id,
    })
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/twin/auth/request", post(post_auth_request))
        .route("/twin/auth/verify",  post(post_auth_verify))
        .route("/twin/auth/session", delete(delete_auth_session))
        .route("/twin/auth/me",      get(get_auth_me))
}