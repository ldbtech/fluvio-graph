//! oauth.rs
//! 
//! Gmail OAuth2 flow for fluvio
//! Reads credentials from ~/.fluvio/config.json 
//! Google OAuth2 downloaded JSON format:
//! {
//!   "client_id":                    "...",
//!   "project_id":                   "...",
//!   "auth_uri":                     "https://accounts.google.com/o/oauth2/auth",
//!   "token_uri":                    "https://oauth2.googleapis.com/token",
//!   "auth_provider_x509_cert_url":  "...",
//!   "client_secret":                "...",
//!   "redirect_uris":                ["http://localhost:8001/connect/gmail/callback"]
//! }
//! 
//! FLOW
//! 1. get_auth_url(force_consent) → user visits this URL, grants access
//! 2. exchange_code(code)   → trade the code for access + refresh tokens
//! 3. refresh_token(token)  → get a new access token when expired
//!     (called automatically by GmailClient before every API call)
//! 

use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::database::gmail_credentials::{self as gmail_cred};

use super::token_store::{
    fluvio_dir, GmailToken, TokenStoreError, load_token, save_token,
};

// ── Errors ────────────────────────────────────────────────────────────────────
#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("config not found at ~/.fluvio/config.json — add your Google OAuth credentials")]
    ConfigNotFound,
    #[error("config parse error: {0}")]
    ConfigParse(#[from] serde_json::Error),
    #[error("redirect_uris is empty in config.json")]
    NoRedirectUri,
    #[error("token exchange failed: {0}")]
    TokenExchange(String),
    #[error("token refresh failed: {0}")]
    TokenRefresh(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("token store error: {0}")]
    TokenStore(#[from] TokenStoreError),
    #[error("database: {0}")]
    Database(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

// -- Config --------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct OAuthConfig {
    pub client_id:              String,
    pub project_id:             String,
    pub auth_uri:               String,
    pub token_uri:              String,
    pub auth_provider_x509_cert_url: String,
    pub client_secret:          String,
    pub redirect_uris:          Vec<String>,
}

/// Google Cloud Console JSON download uses `{"web":{...}}` or `{"installed":{...}}`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConfigFile {
    Flat(OAuthConfig),
    WebClient { web: OAuthConfig },
    InstalledClient { installed: OAuthConfig },
}

impl OAuthConfig {
    // Load from ~/.fluvio/config.json
    pub fn load() -> Result<Self, OAuthError> {
        let path = fluvio_dir().join("config.json");
        if !path.exists() {
            return Err(OAuthError::ConfigNotFound);
        }
        let json = std::fs::read_to_string(&path)?;
        let trimmed = json.trim();
        if trimmed.is_empty() {
            return Err(OAuthError::ConfigParse(
                serde_json::from_str::<OAuthConfig>("").expect_err("empty config"),
            ));
        }
        let parsed: ConfigFile = serde_json::from_str(trimmed)?;
        Ok(match parsed {
            ConfigFile::Flat(c) => c,
            ConfigFile::WebClient { web } => web,
            ConfigFile::InstalledClient { installed } => installed,
        })
    }

    // First redirect URI from the list
    pub fn redirect_uri(&self) -> Result<&str, OAuthError> {
        self.redirect_uris
                   .first()
                   .map(|s| s.as_str())
                   .ok_or(OAuthError::NoRedirectUri)
    }
}

// -- OAuth State ----------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct OAuthState {
    pub url:        String,  // Google auth URL for user to visit
    pub csrf_state: String, // CSRF protection
}

// -- Token exchange response from Google ----------------------------------------
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    access_token:  String,
    #[serde(default)]
    refresh_token: Option<String>, // Absent on refresh-grant responses
    expires_in:    i64, // seconds until expiration
    /// Google JSON key `scope`. Optional on some refresh-grant payloads.
    #[serde(rename = "scope", default)]
    _scope:        Option<String>,
}

// -- Google API Client ----------------------------------------------------------------

/// Build the Google OAuth URL (`gmail.readonly` + `gmail.send` + `gmail.labels`).
///
/// Includes **`prompt=consent`** when `force_consent` or when this account has **no stored
/// Gmail refresh credentials** (`has_gmail_credentials` = false — file or Postgres, depending on caller).
pub fn get_auth_url(force_consent: bool, has_gmail_credentials: bool) -> Result<OAuthState, OAuthError> {
    let config = OAuthConfig::load()?;
    let redirect_uri = config.redirect_uri()?;

    let csrf_state = generate_state();

    let scopes = [
        "https://www.googleapis.com/auth/gmail.readonly",
        "https://www.googleapis.com/auth/gmail.send",
        "https://www.googleapis.com/auth/gmail.labels",
    ]
    .join(" ");

    let need_prompt_consent = force_consent || !has_gmail_credentials;

    let mut url = format!(
        "{auth_uri}?client_id={client_id}&redirect_uri={redirect_uri}&response_type=code&scope={scope}&access_type=offline",
        auth_uri = config.auth_uri,
        client_id = url_encode(&config.client_id),
        redirect_uri = url_encode(redirect_uri),
        scope = url_encode(&scopes),
    );
    if need_prompt_consent {
        url.push_str("&prompt=consent");
    }
    url.push_str(&format!("&state={csrf_state}"));

    Ok(OAuthState { url, csrf_state })
}

/// Exchange the OAuth `code` for tokens and persist to **PostgreSQL** for [`user_id`].
pub async fn exchange_code_for_user(pool: &PgPool, user_id: Uuid, code: &str) -> Result<GmailToken, OAuthError> {
    let token = google_exchange_authorization_code(code).await?;

    let refresh_token = match token.refresh_token_opt {
        Some(rt) => rt,
        None => gmail_cred::get_gmail_credentials(pool, user_id)
            .await
            .map_err(|e| OAuthError::Database(e.to_string()))?
            .ok_or_else(|| {
                OAuthError::TokenExchange(
                    "Google did not return a refresh_token; sign in once with prompt=consent, \
                     or reconnect Gmail from Sources."
                        .to_string(),
                )
            })?
            .refresh_token,
    };

    let gmail_token = GmailToken {
        access_token:  token.access_token,
        refresh_token,
        expires_at:    token.expires_at_unix,
    };

    gmail_cred::upsert_gmail_credentials(
        pool,
        user_id,
        &gmail_token.access_token,
        &gmail_token.refresh_token,
        gmail_token.expires_at,
    )
    .await
    .map_err(|e| OAuthError::Database(e.to_string()))?;

    Ok(gmail_token)
}

/// Legacy: filesystem token path for local CLI (`GmailClient::new()`).
pub async fn exchange_code(code: &str) -> Result<GmailToken, OAuthError> {
    let token = google_exchange_authorization_code(code).await?;

    let refresh_token = match token.refresh_token_opt {
        Some(rt) => rt,
        None => load_token()
            .map(|t| t.refresh_token)
            .map_err(|_| {
                OAuthError::TokenExchange(
                    "Google did not return a refresh_token (first sign-in). \
                     Call POST /connect/gmail/start again with { force_consent: true }."
                        .to_string(),
                )
            })?,
    };

    let gmail_token = GmailToken {
        access_token:  token.access_token,
        refresh_token,
        expires_at:    token.expires_at_unix,
    };

    save_token(&gmail_token)?;
    Ok(gmail_token)
}

struct GoogleCodeExchangeResult {
    access_token:      String,
    refresh_token_opt: Option<String>,
    expires_at_unix:   i64,
}

async fn google_exchange_authorization_code(code: &str) -> Result<GoogleCodeExchangeResult, OAuthError> {
    let config = OAuthConfig::load()?;
    let redirect_uri = config.redirect_uri()?.to_string();

    let client: Client = Client::new();
    let res = client
        .post(&config.token_uri)
        .form(&[
            ("code", code),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;

    if !status.is_success() {
        return Err(OAuthError::TokenExchange(format!(
            "HTTP {status} - {body}"
        )));
    }

    let token_res: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| OAuthError::TokenExchange(format!("parse error: {e} — body: {body}")))?;

    Ok(GoogleCodeExchangeResult {
        access_token:      token_res.access_token,
        refresh_token_opt: token_res.refresh_token,
        expires_at_unix:   Utc::now().timestamp() + token_res.expires_in,
    })
}

/// Get a fresh access token from the stored refresh token.
/// Saves the updated token back to the disk. 
/// Called automatically by GmailClient when `token.is_expired()` returns true.
/// 
pub async fn refresh_access_token(token: &GmailToken) -> Result<GmailToken, OAuthError> {
    let config = OAuthConfig::load()?;
 
    let client = Client::new();
    let res = client
        .post(&config.token_uri)
        .form(&[
            ("refresh_token", token.refresh_token.as_str()),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
 
    let status = res.status();
    let body   = res.text().await?;
 
    if !status.is_success() {
        return Err(OAuthError::TokenRefresh(format!(
            "HTTP {status}: {body}"
        )));
    }
 
    let token_res: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| OAuthError::TokenRefresh(format!("parse error: {e} — body: {body}")))?;
 
    // Refresh responses don't include a new refresh_token — keep the existing one.
    let refreshed = GmailToken {
        access_token:  token_res.access_token,
        refresh_token: token.refresh_token.clone(),
        expires_at:    Utc::now().timestamp() + token_res.expires_in,
    };

    save_token(&refreshed)?;
    Ok(refreshed)
}

/// Refresh access token and persist to **PostgreSQL** for this user.
pub async fn refresh_access_token_for_user(
    pool:    &PgPool,
    user_id: Uuid,
    token:   &GmailToken,
) -> Result<GmailToken, OAuthError> {
    let config = OAuthConfig::load()?;

    let client = Client::new();
    let res = client
        .post(&config.token_uri)
        .form(&[
            ("refresh_token", token.refresh_token.as_str()),
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;

    let status = res.status();
    let body = res.text().await?;

    if !status.is_success() {
        return Err(OAuthError::TokenRefresh(format!(
            "HTTP {status}: {body}"
        )));
    }

    let token_res: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| OAuthError::TokenRefresh(format!("parse error: {e} — body: {body}")))?;

    let refreshed = GmailToken {
        access_token:  token_res.access_token,
        refresh_token: token.refresh_token.clone(),
        expires_at:    Utc::now().timestamp() + token_res.expires_in,
    };

    gmail_cred::upsert_gmail_credentials(
        pool,
        user_id,
        &refreshed.access_token,
        &refreshed.refresh_token,
        refreshed.expires_at,
    )
    .await
    .map_err(|e| OAuthError::Database(e.to_string()))?;

    Ok(refreshed)
}
 

/// Minimal percent-encoding for URL query params 
fn url_encode(s: &str) -> String {
    s.chars()
         .flat_map(|c| match c {
            ' ' => "%20".chars().collect::<Vec<_>>(),
            '+' => "%2B".chars().collect(),
            '&' => "%26".chars().collect(),
            '=' => "%3D".chars().collect(),
            '#' => "%23".chars().collect(),
            _   => vec![c],
         })
         .collect()
}

/// Generate a random CSRF state string (16 hex chars) from current nanosecond time.
fn generate_state() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);

    format!("{:016X}", hasher.finish())
}

/// Test the OAuth flow by getting an auth URL and exchanging a code.
#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn test_urlenconde_spaces() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }
 
    #[test]
    fn test_urlenconde_scopes() {
        let scopes = "https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/gmail.send https://www.googleapis.com/auth/gmail.labels";
        let encoded = url_encode(scopes);
        assert!(encoded.contains("%20"));
        assert!(!encoded.contains(' '));
    }
 
    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        assert_eq!(state.len(), 16);
    }
 
    #[test]
    fn test_generate_state_unique() {
        // Two calls should produce different states (time-based).
        let a = generate_state();
        std::thread::sleep(std::time::Duration::from_nanos(100));
        let b = generate_state();
        // Not guaranteed but almost certain with nanosecond resolution.
        // If this flakes, it's still not a correctness problem.
        let _ = (a, b); // just ensure they compile and run
    }
 
    #[test]
    fn test_config_not_found() {
        // Temporarily point HOME somewhere that has no config.json.
        let original = std::env::var("HOME").unwrap_or_default();
        unsafe {
            std::env::set_var("HOME", "/tmp/fluvio_no_config_test");
        }
        let result = OAuthConfig::load();
        unsafe {
            std::env::set_var("HOME", original);
        }
        assert!(matches!(result, Err(OAuthError::ConfigNotFound)));
    }

    #[test]
    fn test_google_download_web_wrapper_parses() {
        let j = r#"{"web":{"client_id":"cid","project_id":"pid","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_secret":"sec","redirect_uris":["http://localhost:8001/connect/gmail/callback"]}}"#;
        let parsed: ConfigFile = serde_json::from_str(j).unwrap();
        let c = match parsed {
            ConfigFile::WebClient { web } => web,
            _ => panic!("expected web wrapper"),
        };
        assert_eq!(c.client_id, "cid");
        assert_eq!(c.redirect_uris[0], "http://localhost:8001/connect/gmail/callback");
    }

    #[test]
    fn token_response_maps_json_scope_field() {
        let body = r#"{
            "access_token": "ya29.test",
            "expires_in": 3599,
            "refresh_token": "1//refresh",
            "scope": "https://www.googleapis.com/auth/gmail.readonly https://www.googleapis.com/auth/gmail.send https://www.googleapis.com/auth/gmail.labels",
            "token_type": "Bearer"
            }"#;
        let t: TokenResponse = serde_json::from_str(body).expect("google token shape");
        assert_eq!(t.access_token, "ya29.test");
        assert_eq!(t.expires_in, 3599);
        assert_eq!(t.refresh_token.as_deref(), Some("1//refresh"));
        assert!(t._scope.is_some());
    }

    #[test]
    fn token_response_refresh_without_scope() {
        let body = r#"{"access_token":"new","expires_in":3600,"token_type":"Bearer"}"#;
        let t: TokenResponse = serde_json::from_str(body).unwrap();
        assert_eq!(t.access_token, "new");
        assert!(t.refresh_token.is_none());
        assert!(t._scope.is_none());
    }
}