//! gmail.rs
//!
//! Gmail API HTTP client.
//! All calls go to https://gmail.googleapis.com/gmail/v1/users/me/
//!
//! Handles:
//!   - Token refresh automatically when access token is expired
//!   - Pagination via next_page_token
//!   - Rate limit retry (429) with exponential backoff
//!   - Long timeouts + retries on transient transport errors (large `threads?format=full` payloads)
//!   - All errors surfaced as GmailClientError

use std::error::Error as StdError;
use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::ingestion_registry::email::auth::{
    load_token, refresh_access_token, GmailToken,
};

use super::models::{
    GmailLabel, GmailMessage, GmailThread,
    HistoryListResponse, LabelListResponse,
    MessageListResponse, ThreadListResponse,
};

const BASE_URL: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const MAX_RETRIES: u32 = 3;
/// Full threads/messages can be multi‑MB JSON; default reqwest timeout (30s) is often too low.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);
const TRANSPORT_RETRY_MAX: u32 = 2;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum GmailClientError {
    #[error("not authenticated — no token at ~/.fluvio/credentials/gmail.json; complete OAuth via http://localhost:8001/connect/gmail?redirect=1 first")]
    NotAuthenticated,
    #[error("token refresh failed: {0}")]
    TokenRefresh(String),
    #[error("API error {status}: {body}")]
    ApiError { status: u16, body: String },
    #[error("rate limited — max retries exceeded")]
    RateLimited,
    #[error("deserialize error: {0}")]
    Deserialize(#[from] serde_json::Error),
    /// Transport failure before a response (timeout, DNS, TLS, connection reset, etc.).
    #[error("HTTP error: {0}")]
    Http(String),
}

fn format_reqwest_error(e: &reqwest::Error) -> String {
    let mut s = e.to_string();
    if e.is_timeout() {
        s.push_str(" [timeout]");
    }
    if e.is_connect() {
        s.push_str(" [connect]");
    }
    if let Some(url) = e.url() {
        s.push_str(&format!(" [url: {url}]"));
    }
    let mut src = e.source();
    let mut n = 0u8;
    while let Some(err) = src {
        s.push_str(&format!("; {err}"));
        src = err.source();
        n += 1;
        if n >= 4 {
            break;
        }
    }
    s
}

impl From<reqwest::Error> for GmailClientError {
    fn from(e: reqwest::Error) -> Self {
        GmailClientError::Http(format_reqwest_error(&e))
    }
}

// ── Client ────────────────────────────────────────────────────────────────────

pub struct GmailClient {
    http:  Client,
    token: GmailToken,
}

impl GmailClient {
    /// Load credentials from ~/.fluvio/credentials/gmail.json and build client.
    /// Automatically refreshes the token if it is expired.
    pub async fn new() -> Result<Self, GmailClientError> {
        let token = load_token().map_err(|_| GmailClientError::NotAuthenticated)?;

        let token = if token.is_expired() {
            refresh_access_token(&token)
                .await
                .map_err(|e| GmailClientError::TokenRefresh(e.to_string()))?
        } else {
            token
        };

        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(concat!("kg-engine/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| GmailClientError::Http(format_reqwest_error(&e)))?;

        Ok(Self { http, token })
    }

    // ── Messages ──────────────────────────────────────────────────────────────

    /// List message IDs matching an optional query string.
    /// Returns all pages — callers get the full list at once.
    ///
    /// `query` uses Gmail search syntax e.g. "after:2024/01/01 -in:spam"
    pub async fn list_messages(
        &mut self,
        query: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<Vec<super::models::MessageRef>, GmailClientError> {
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut params: Vec<(&str, String)> = Vec::new();

            if let Some(q) = query {
                params.push(("q", q.to_string()));
            }
            if let Some(max) = max_results {
                params.push(("maxResults", max.to_string()));
            }
            if let Some(ref pt) = page_token {
                params.push(("pageToken", pt.clone()));
            }

            let res: MessageListResponse = self
                .get("messages", &params)
                .await?;

            all.extend(res.messages);

            match res.next_page_token {
                Some(pt) => page_token = Some(pt),
                None     => break,
            }

            // Stop early if caller set max_results and we have enough.
            if let Some(max) = max_results {
                if all.len() >= max as usize {
                    all.truncate(max as usize);
                    break;
                }
            }
        }

        Ok(all)
    }

    /// Fetch a single full message by ID.
    pub async fn get_message(
        &mut self,
        id: &str,
    ) -> Result<GmailMessage, GmailClientError> {
        let path = format!("messages/{}", id);
        self.get(&path, &[("format", "full".to_string())]).await
    }

    /// Fetch multiple messages by ID concurrently in batches.
    /// Useful after `list_messages` gives you IDs — fetch details in parallel.
    pub async fn get_messages_batch(
        &mut self,
        ids: &[String],
        batch_size: usize,
    ) -> Result<Vec<GmailMessage>, GmailClientError> {
        let mut results = Vec::with_capacity(ids.len());

        for chunk in ids.chunks(batch_size) {
            let mut handles = Vec::new();

            for id in chunk {
                // Clone what we need to move into the async block.
                let http    = self.http.clone();
                let token   = self.token.access_token.clone();
                let message_id = id.clone();

                handles.push(tokio::spawn(async move {
                    let url = format!(
                        "{}/messages/{}?format=full",
                        BASE_URL, message_id
                    );
                    let res = http
                        .get(&url)
                        .bearer_auth(&token)
                        .send()
                        .await?;
                    let body = res.text().await?;
                    let msg: GmailMessage =
                        serde_json::from_str(&body).map_err(GmailClientError::from)?;
                    Ok::<GmailMessage, GmailClientError>(msg)
                }));
            }

            for handle in handles {
                match handle.await {
                    Ok(Ok(msg)) => results.push(msg),
                    Ok(Err(e)) => return Err(e),
                    Err(_) => {} // task panicked — skip
                }
            }
        }

        Ok(results)
    }

    // ── Threads ───────────────────────────────────────────────────────────────

    /// List thread references.
    pub async fn list_threads(
        &mut self,
        query: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<Vec<super::models::ThreadRef>, GmailClientError> {
        let mut all = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut params: Vec<(&str, String)> = Vec::new();
            if let Some(q) = query { params.push(("q", q.to_string())); }
            if let Some(max) = max_results { params.push(("maxResults", max.to_string())); }
            if let Some(ref pt) = page_token { params.push(("pageToken", pt.clone())); }

            let res: ThreadListResponse = self.get("threads", &params).await?;
            all.extend(res.threads);

            match res.next_page_token {
                Some(pt) => page_token = Some(pt),
                None     => break,
            }

            if let Some(max) = max_results {
                if all.len() >= max as usize {
                    all.truncate(max as usize);
                    break;
                }
            }
        }

        Ok(all)
    }

    /// Fetch a full thread with all its messages.
    pub async fn get_thread(
        &mut self,
        id: &str,
    ) -> Result<GmailThread, GmailClientError> {
        let path = format!("threads/{}", id);
        self.get(&path, &[("format", "full".to_string())]).await
    }

    // ── Labels ────────────────────────────────────────────────────────────────

    /// Fetch all labels for the authenticated user.
    pub async fn list_labels(&mut self) -> Result<Vec<GmailLabel>, GmailClientError> {
        let res: LabelListResponse = self.get("labels", &[]).await?;
        Ok(res.labels)
    }

    // ── Incremental sync ──────────────────────────────────────────────────────

    /// Fetch history records since a given historyId.
    /// Used by incremental sync to find only new/changed messages.
    pub async fn list_history(
        &mut self,
        start_history_id: &str,
        page_token: Option<&str>,
    ) -> Result<HistoryListResponse, GmailClientError> {
        let mut params = vec![
            ("startHistoryId", start_history_id.to_string()),
            ("historyTypes",   "messageAdded".to_string()),
            ("historyTypes",   "messageDeleted".to_string()),
        ];
        if let Some(pt) = page_token {
            params.push(("pageToken", pt.to_string()));
        }
        self.get("history", &params).await
    }

    // ── Core HTTP ─────────────────────────────────────────────────────────────

    /// GET `BASE_URL/{path}` with query params.
    /// Refreshes token on 401. Retries on 429 with exponential backoff.
    async fn get<T: DeserializeOwned>(
        &mut self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T, GmailClientError> {
        let url = format!("{}/{}", BASE_URL, path);
        let mut attempt = 0u32;

        loop {
            let mut send_try = 0u32;
            let res = loop {
                let mut req = self.http.get(&url).bearer_auth(&self.token.access_token);

                for (k, v) in params {
                    req = req.query(&[(k, v)]);
                }

                match req.send().await {
                    Ok(r) => break r,
                    Err(e)
                        if send_try < TRANSPORT_RETRY_MAX
                            && (e.is_timeout() || e.is_connect()) =>
                    {
                        let wait = Duration::from_millis(400 * 2u64.pow(send_try));
                        tokio::time::sleep(wait).await;
                        send_try += 1;
                    }
                    Err(e) => return Err(GmailClientError::Http(format_reqwest_error(&e))),
                }
            };
            let status = res.status();

            // 401 — refresh token and retry once.
            if status == 401 && attempt == 0 {
                self.token = refresh_access_token(&self.token)
                    .await
                    .map_err(|e| GmailClientError::TokenRefresh(e.to_string()))?;
                attempt += 1;
                continue;
            }

            // 429 — rate limited, exponential backoff.
            if status == 429 {
                if attempt >= MAX_RETRIES {
                    return Err(GmailClientError::RateLimited);
                }
                let wait = Duration::from_millis(500 * 2u64.pow(attempt));
                tokio::time::sleep(wait).await;
                attempt += 1;
                continue;
            }

            // Any other non-2xx.
            if !status.is_success() {
                let body = res.text().await.unwrap_or_default();
                return Err(GmailClientError::ApiError {
                    status: status.as_u16(),
                    body,
                });
            }

            let body = res.text().await?;
            let parsed: T = serde_json::from_str(&body)?;
            return Ok(parsed);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_registry::email::client::models::GmailMessage;

    // All tests here are unit tests against the model layer.
    // Integration tests (real Gmail API calls) live in tests/gmail_integration.rs
    // and are gated behind --ignored so CI doesn't need credentials.

    #[test]
    fn test_base_url_format() {
        let path = "messages/abc123";
        let url  = format!("{}/{}", BASE_URL, path);
        assert_eq!(url, "https://gmail.googleapis.com/gmail/v1/users/me/messages/abc123");
    }

    #[test]
    fn test_backoff_durations() {
        // Verify exponential backoff values are sane.
        for attempt in 0..MAX_RETRIES {
            let wait_ms = 500 * 2u64.pow(attempt);
            assert!(wait_ms <= 4000, "backoff too long at attempt {attempt}: {wait_ms}ms");
        }
    }

    #[test]
    fn test_message_deserialization_via_client_path() {
        // Simulate what the client receives from the API and deserializes.
        let raw = r#"{
            "id": "msg_xyz",
            "threadId": "thread_xyz",
            "labelIds": ["INBOX"],
            "snippet": "Test snippet",
            "internalDate": "1700000000000",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From",    "value": "test@example.com"},
                    {"name": "Subject", "value": "Unit test email"}
                ],
                "body": {"size": 0},
                "parts": []
            }
        }"#;

        let msg: GmailMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.id, "msg_xyz");
        assert_eq!(msg.subject(), Some("Unit test email"));
        assert_eq!(msg.from(), Some("test@example.com"));
        assert_eq!(msg.timestamp_secs(), Some(1_700_000_000));
    }

    /// Integration test — only runs with `cargo test -- --ignored`
    /// Requires ~/.fluvio/credentials/gmail.json to exist.
    #[tokio::test]
    #[ignore]
    async fn integration_list_labels() {
        let mut client = GmailClient::new().await
            .expect("failed to build Gmail client — are credentials set up?");

        let labels = client.list_labels().await
            .expect("failed to list labels");

        assert!(!labels.is_empty(), "expected at least one label");
        println!("Labels ({}):", labels.len());
        for label in &labels {
            println!("  {} — {}", label.id, label.name);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn integration_list_recent_messages() {
        let mut client = GmailClient::new().await.unwrap();

        let messages = client
            .list_messages(Some("newer_than:7d"), Some(5))
            .await
            .unwrap();

        println!("Recent messages ({}): ", messages.len());
        for m in &messages {
            println!("  {} / thread {}", m.id, m.thread_id);
        }
    }
}