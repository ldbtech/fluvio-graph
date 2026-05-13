//! Gmail OAuth, focused inbox preview (`/gmail/recent`), and sender allow-list (`/gmail/focus`).

use std::collections::HashSet;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use crate::app_state::{AppState, GmailOauthPending};
use crate::authentication::AuthUser;
use crate::database::gmail_credentials::gmail_credentials_exist;
use crate::database::gmail_inbox_prefs;
use crate::database::gmail_reply_agent as gmail_agent_db;
use crate::ingestion_registry::email::client::gmail::{GmailClient, GmailClientError};
use crate::ingestion_registry::email::client::models::GmailMessage;
use crate::ingestion_registry::email::{
    auth::{exchange_code_for_user, get_auth_url},
    gmail_query,
    reply_agent,
};

pub fn gmail_router() -> Router<AppState> {
    Router::new()
        .route("/connect/gmail/start", post(connect_gmail_start))
        .route("/connect/gmail/callback", get(connect_gmail_callback))
        .route("/connect/gmail/status", get(connect_gmail_status))
        .route("/gmail/recent", get(gmail_recent_inbox))
        .route("/gmail/focus", get(get_gmail_focus).put(put_gmail_focus))
        .route(
            "/gmail/agent/settings",
            get(get_gmail_agent_settings).put(put_gmail_agent_settings),
        )
        .route("/gmail/agent/reviews", get(get_gmail_agent_reviews))
        .route("/gmail/agent/run", post(post_gmail_agent_run))
}

// ---- POST /connect/gmail/start
#[derive(Deserialize)]
struct ConnectGmailStartBody {
    #[serde(default)]
    force_consent: bool,
}

#[derive(Serialize)]
struct ConnectGmailStartResponse {
    url: String,
}

/// Returns the Google consent URL. Requires a logged-in Fluvio session so tokens are stored under
/// the correct `user_id` in PostgreSQL.
async fn connect_gmail_start(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<ConnectGmailStartBody>,
) -> Result<Json<ConnectGmailStartResponse>, (StatusCode, String)> {
    let has_cred = gmail_credentials_exist(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let oauth =
        get_auth_url(body.force_consent, has_cred).map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

    *state.oauth_gmail.lock().unwrap() = Some(GmailOauthPending {
        csrf_state: oauth.csrf_state.clone(),
        user_id:    user.id,
    });

    Ok(Json(ConnectGmailStartResponse { url: oauth.url }))
}

// ---- GET /connect/gmail/status
async fn connect_gmail_status(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let connected = gmail_credentials_exist(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "connected": connected })))
}

// ---- GET /connect/gmail/callback
#[derive(Deserialize)]
struct GmailCallbackQuery {
    code:  Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn connect_gmail_callback(
    State(state): State<AppState>,
    Query(q): Query<GmailCallbackQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    if let Some(err) = &q.error {
        let desc = q
            .error_description
            .as_deref()
            .unwrap_or("")
            .replace('<', "")
            .replace('>', "");
        return Ok(Html(format!(
            "<!DOCTYPE html><html><body style=\"font-family:system-ui;padding:2rem\">\
             <h1>OAuth error</h1><p><code>{}</code></p><p>{}</p></body></html>",
            err, desc
        )));
    }

    let code = q
        .code
        .ok_or((StatusCode::BAD_REQUEST, "missing code".to_string()))?;
    let state_param = q
        .state
        .ok_or((StatusCode::BAD_REQUEST, "missing state".to_string()))?;

    let pending = state
        .oauth_gmail
        .lock()
        .unwrap()
        .take()
        .ok_or((
            StatusCode::BAD_REQUEST,
            "OAuth session expired — start again from Sources → Connect Gmail".to_string(),
        ))?;

    if pending.csrf_state != state_param {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid OAuth state — start Gmail connect again from the app".to_string(),
        ));
    }

    exchange_code_for_user(&state.pg_pool, pending.user_id, &code)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Html(format!(
        "<!DOCTYPE html><html><body style=\"font-family:system-ui;padding:2rem\">\
         <h1>Gmail connected</h1>\
         <p>Token saved for your account in <strong>PostgreSQL</strong>.</p>\
         <p>Close this tab — your inbox will show on the dashboard.</p>\
         <p style=\"color:#666;font-size:14px\">User <code>{}</code></p>\
         </body></html>",
        pending.user_id,
    )))
}

// ---- GET /gmail/recent
#[derive(Deserialize)]
struct GmailRecentQuery {
    #[serde(default = "default_gmail_recent_limit")]
    limit: u32,
}

fn default_gmail_recent_limit() -> u32 {
    10
}

#[derive(Serialize)]
struct GmailRecentMailJson {
    id:               String,
    thread_id:        String,
    snippet:          Option<String>,
    subject:          Option<String>,
    from:             Option<String>,
    date_header:      Option<String>,
    internal_date_ms: Option<i64>,
    /// Set when this row came from a Gmail History `messagesAdded` delta this poll.
    #[serde(skip_serializing_if = "Option::is_none")]
    is_new:           Option<bool>,
}

fn gmail_recent_map_err(e: GmailClientError) -> (StatusCode, String) {
    match e {
        GmailClientError::NotAuthenticated => (
            StatusCode::FORBIDDEN,
            "Gmail not connected — connect Gmail from Sources.".to_string(),
        ),
        GmailClientError::TokenRefresh(msg) => (StatusCode::BAD_GATEWAY, msg),
        GmailClientError::ApiError { status, body } => (
            StatusCode::BAD_GATEWAY,
            format!(
                "Gmail API HTTP {status}: {}",
                body.chars().take(400).collect::<String>()
            ),
        ),
        GmailClientError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, e.to_string()),
        GmailClientError::Deserialize(err) => (StatusCode::BAD_GATEWAY, err.to_string()),
        GmailClientError::Http(msg) => (StatusCode::BAD_GATEWAY, msg),
    }
}

fn map_gmail_msg(m: GmailMessage, is_new: bool) -> GmailRecentMailJson {
    let subject = m.subject().map(str::to_string);
    let from = m.from().map(str::to_string);
    let date_header = m.date().map(str::to_string);
    GmailRecentMailJson {
        id:               m.id,
        thread_id:        m.thread_id,
        snippet:          m.snippet,
        subject,
        from,
        date_header,
        internal_date_ms: m.internal_date,
        is_new:           if is_new { Some(true) } else { None },
    }
}

async fn gmail_recent_inbox(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<GmailRecentQuery>,
) -> Result<Json<Vec<GmailRecentMailJson>>, (StatusCode, String)> {
    let lim = q.limit.clamp(1, 50);
    let pool = &state.pg_pool;
    let uid = user.id;

    let focus =
        gmail_inbox_prefs::list_focus_senders(pool, uid)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let gmail_q = gmail_query::inbox_recent_list_q(&focus);

    let mut client = GmailClient::for_user(pool, uid).await.map_err(gmail_recent_map_err)?;

    let profile = client.get_user_profile().await.map_err(gmail_recent_map_err)?;
    let prev = gmail_inbox_prefs::get_history_cursor(pool, uid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut delta_ids: Vec<String> = Vec::new();
    if let Some(ref old_h) = prev {
        if old_h != &profile.history_id {
            match client.collect_message_ids_added_since(old_h, 400).await {
                Ok(ids) => delta_ids = ids,
                Err(GmailClientError::ApiError { status: 404, .. }) => delta_ids.clear(),
                Err(e) => return Err(gmail_recent_map_err(e)),
            }
        }
    }

    let mut delta_msgs: Vec<GmailMessage> = Vec::new();
    for id in &delta_ids {
        let Ok(m) = client.get_message_metadata(id).await else {
            continue;
        };
        if !m.label_ids.iter().any(|l| l == "INBOX") {
            continue;
        }
        if !gmail_query::from_header_matches_focus(m.from(), &focus) {
            continue;
        }
        delta_msgs.push(m);
    }
    delta_msgs.sort_by(|a, b| {
        let ta = a.internal_date.unwrap_or(0);
        let tb = b.internal_date.unwrap_or(0);
        tb.cmp(&ta)
    });

    let baseline = client
        .inbox_recent_summaries(lim, &gmail_q)
        .await
        .map_err(gmail_recent_map_err)?;

    gmail_inbox_prefs::set_history_cursor(pool, uid, profile.history_id.as_str())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut seen = HashSet::new();
    let mut merged: Vec<(GmailMessage, bool)> = Vec::new();
    for m in delta_msgs {
        if seen.insert(m.id.clone()) {
            merged.push((m, true));
        }
    }
    for m in baseline {
        if seen.insert(m.id.clone()) {
            merged.push((m, false));
        }
    }
    merged.truncate(lim as usize);

    let rows = merged.into_iter().map(|(m, dn)| map_gmail_msg(m, dn)).collect();
    Ok(Json(rows))
}

// ---- GET / PUT /gmail/focus
#[derive(Deserialize)]
struct GmailFocusBody {
    senders: Vec<String>,
}

async fn get_gmail_focus(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let senders = gmail_inbox_prefs::list_focus_senders(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "senders": senders })))
}

async fn put_gmail_focus(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<GmailFocusBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let senders = gmail_inbox_prefs::replace_focus_senders(&state.pg_pool, user.id, &body.senders)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "senders": senders })))
}

// ---- GET / PUT /gmail/agent/settings
async fn get_gmail_agent_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let r = gmail_agent_db::get_reply_agent_settings(&state.pg_pool, user.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "send_mode": r.send_mode.as_db(),
        "context_sources": r.context_sources,
        "updated_at": r.updated_at,
    })))
}

#[derive(Deserialize)]
struct GmailAgentPutBody {
    #[serde(default)]
    send_mode:       Option<String>,
    #[serde(default)]
    context_sources: gmail_agent_db::GmailAgentContextSources,
}

async fn put_gmail_agent_settings(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<GmailAgentPutBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mode_str = body
        .send_mode
        .as_deref()
        .unwrap_or(gmail_agent_db::GmailAgentSendMode::AlwaysReview.as_db());
    if mode_str != gmail_agent_db::GmailAgentSendMode::AlwaysReview.as_db()
        && mode_str != gmail_agent_db::GmailAgentSendMode::AutoWhenConfident.as_db()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "send_mode must be \"always_review\" or \"auto_when_confident\"".into(),
        ));
    }
    let mode = gmail_agent_db::GmailAgentSendMode::from_db(mode_str);
    let r = gmail_agent_db::put_reply_agent_settings(&state.pg_pool, user.id, &mode, body.context_sources)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({
        "send_mode": r.send_mode.as_db(),
        "context_sources": r.context_sources,
        "updated_at": r.updated_at,
    })))
}

#[derive(Deserialize)]
struct GmailAgentReviewsQuery {
    #[serde(default = "default_review_limit")]
    limit: i64,
}

fn default_review_limit() -> i64 {
    80
}

async fn get_gmail_agent_reviews(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Query(q): Query<GmailAgentReviewsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let rows =
        gmail_agent_db::list_agent_review_drafts(&state.pg_pool, user.id, q.limit.max(1).min(200))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "items": rows })))
}

// ---- POST /gmail/agent/run
#[derive(Deserialize)]
struct GmailAgentRunBody {
    #[serde(default)]
    dry_run:          bool,
    #[serde(default = "default_gmail_agent_max_candidates")]
    max_candidates: u32,
}

fn default_gmail_agent_max_candidates() -> u32 {
    5
}

async fn post_gmail_agent_run(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(body): Json<GmailAgentRunBody>,
) -> Result<Json<reply_agent::GmailAgentCycleResponse>, (StatusCode, String)> {
    let out = reply_agent::run_gmail_agent_cycle(&state, &user, body.dry_run, body.max_candidates)
    .await?;
    Ok(Json(out))
}
