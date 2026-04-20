//! connector.rs
//!
//! GmailConnector — implements FluvioConnector for Gmail.
//!
//! This is the public entry point for the email domain.
//! It wires together: auth → client → sync mode → normalizer → NormalizedChunk
//!
//! Usage from CLI or server:
//!   let connector = GmailConnector::new();
//!   let chunks    = connector.extract("full")?;        // full thread sync
//!   let chunks    = connector.extract("incremental")?; // since last sync
//!   let chunks    = connector.extract("newer_than:7d")?; // Gmail query
//!
//! Optional limits (see `with_max_threads`, `with_max_message_fetch`, `with_thread_query`,
//! `with_bootstrap_query`): cap how many threads or messages are listed per run, scope full
//! sync with a Gmail `q` string, and on first incremental (no stored historyId) run a bounded
//! bootstrap list instead of listing the entire mailbox.

use std::sync::Arc;

use crate::graph::enums::Domain;
use crate::ingestion_registry::connector::{ConnectorError, FluvioConnector, NormalizedChunk};

use super::auth::{credentials_exist, gmail_token_path};
use super::client::gmail::GmailClient;
use super::normalizer::{normalize_labels, normalize_message, normalize_thread};
use super::sync::progress::GmailSyncProgress;
use super::sync::state::SyncState;

// ── Sync mode ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SyncMode {
    /// Pull everything — ignores historyId.
    Full,
    /// Pull only changes since the last stored historyId.
    Incremental,
    /// Pass a raw Gmail search query e.g. "newer_than:7d -in:spam".
    Query(String),
}

impl SyncMode {
    fn from_source(source: &str) -> Self {
        match source.trim() {
            "full"        => SyncMode::Full,
            "incremental" => SyncMode::Incremental,
            other         => SyncMode::Query(other.to_string()),
        }
    }
}

// ── Connector ─────────────────────────────────────────────────────────────────

pub struct GmailConnector {
    /// Max thread refs from `users.threads.list` in full / bootstrap paths (`None` = unlimited).
    pub max_threads: Option<u32>,
    /// Max message refs from `users.messages.list` (query mode) and max **new** messages to pull
    /// per incremental run (`None` = unlimited). If the cap is hit mid-history, `historyId` is not
    /// advanced so the next run can continue.
    pub max_messages: Option<u32>,
    /// Gmail search `q` passed to `threads.list` for full sync and for incremental fallback when
    /// no `bootstrap_query` is set but you still want a scoped first sync.
    pub thread_query: Option<String>,
    /// When mode is incremental and there is no stored `historyId`, list threads with this `q`
    /// instead of syncing the whole mailbox. If unset, falls back to `thread_query`, then unscoped full.
    pub bootstrap_query: Option<String>,
    /// Batch size for concurrent message fetching.
    pub batch_size: usize,
    /// When set (e.g. from the HTTP server), thread/message fetch progress is recorded for polling.
    progress: Option<Arc<GmailSyncProgress>>,
}

impl GmailConnector {
    pub fn new() -> Self {
        Self {
            max_threads:      None,
            max_messages:     None,
            thread_query:     None,
            bootstrap_query:  None,
            batch_size:       10,
            progress:         None,
        }
    }

    /// Sets both [`Self::max_threads`] and [`Self::max_messages`] (handy for CLI/tests).
    pub fn with_max_messages(mut self, max: u32) -> Self {
        self.max_threads = Some(max);
        self.max_messages = Some(max);
        self
    }

    pub fn with_max_threads(mut self, max: u32) -> Self {
        self.max_threads = Some(max);
        self
    }

    pub fn with_max_message_fetch(mut self, max: u32) -> Self {
        self.max_messages = Some(max);
        self
    }

    pub fn with_thread_query(mut self, q: impl Into<String>) -> Self {
        self.thread_query = Some(q.into());
        self
    }

    pub fn with_bootstrap_query(mut self, q: impl Into<String>) -> Self {
        self.bootstrap_query = Some(q.into());
        self
    }

    pub fn with_progress(mut self, progress: Arc<GmailSyncProgress>) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Check credentials exist before attempting any API calls.
    fn check_auth(&self) -> Result<(), ConnectorError> {
        if !credentials_exist() {
            let path = gmail_token_path().display().to_string();
            return Err(ConnectorError::Auth(format!(
                "Gmail OAuth token not found (expected {path}). \
                 Sign in: visit http://localhost:8001/connect/gmail?redirect=1 (same machine as kg-engine) \
                 or use the web app Sources → Gmail → Sign in with Google, finish the Google consent page, \
                 then try sync again."
            )));
        }
        Ok(())
    }

    /// Full sync: list threads (optional Gmail `q` + cap), normalize each thread.
    async fn sync_full(
        &self,
        client: &mut GmailClient,
    ) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        self.sync_threads_scoped(client, self.thread_query.as_deref(), "full")
            .await
    }

    /// Thread-based sync shared by full mode and incremental bootstrap.
    async fn sync_threads_scoped(
        &self,
        client: &mut GmailClient,
        list_q: Option<&str>,
        label: &str,
    ) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        let mut chunks = Vec::new();

        if let Some(p) = &self.progress {
            p.set_listing_labels();
        }

        let labels = client.list_labels().await
            .map_err(|e| ConnectorError::Api(e.to_string()))?;
        chunks.extend(normalize_labels(&labels));

        if let Some(p) = &self.progress {
            p.set_listing_threads();
        }

        let thread_refs = client
            .list_threads(list_q, self.max_threads)
            .await
            .map_err(|e| ConnectorError::Api(e.to_string()))?;

        if let Some(p) = &self.progress {
            p.set_thread_totals(thread_refs.len());
        }

        let q_desc = list_q.unwrap_or("(no query)");
        println!(
            "[gmail] {} sync: {} threads (q={}), fetching...",
            label,
            thread_refs.len(),
            q_desc
        );

        let total_threads = thread_refs.len();

        for (i, thread_ref) in thread_refs.iter().enumerate() {
            let thread = client.get_thread(&thread_ref.id).await
                .map_err(|e| ConnectorError::Api(e.to_string()))?;

            let start_index = chunks.len();
            let thread_chunks = normalize_thread(&thread, start_index);
            chunks.extend(thread_chunks);

            if let Some(p) = &self.progress {
                let done = i + 1;
                if total_threads == 0 || done % 20 == 0 || done == total_threads {
                    p.set_thread_progress(done, total_threads, chunks.len());
                }
            }

            if (i + 1) % 50 == 0 {
                println!("[gmail] Fetched {}/{} threads", i + 1, thread_refs.len());
            }
        }

        if let Some(last_thread) = thread_refs.last() {
            if let Ok(thread) = client.get_thread(&last_thread.id).await {
                if let Some(history_id) = thread.history_id {
                    let mut state = SyncState::load().unwrap_or_default();
                    state.history_id    = Some(history_id);
                    state.total_synced  += chunks.len();
                    let _ = state.save();
                }
            }
        }

        println!("[gmail] {} sync complete: {} chunks", label, chunks.len());
        Ok(chunks)
    }

    /// Incremental sync: fetch only messages added since last historyId.
    async fn sync_incremental(
        &self,
        client: &mut GmailClient,
    ) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        let state = SyncState::load().unwrap_or_default();

        let history_id = match &state.history_id {
            Some(id) => id.clone(),
            None => {
                if let Some(ref bq) = self.bootstrap_query {
                    println!(
                        "[gmail] No prior sync state — bootstrap threads with q={:?}",
                        bq
                    );
                    return self
                        .sync_threads_scoped(client, Some(bq.as_str()), "bootstrap")
                        .await;
                }
                if let Some(ref tq) = self.thread_query {
                    println!(
                        "[gmail] No prior sync state — scoped threads with thread_query={:?}",
                        tq
                    );
                    return self
                        .sync_threads_scoped(client, Some(tq.as_str()), "bootstrap")
                        .await;
                }
                println!("[gmail] No prior sync state — full mailbox thread sync");
                return self.sync_full(client).await;
            }
        };

        println!("[gmail] Incremental sync from historyId {}", history_id);

        if let Some(p) = &self.progress {
            p.set_indeterminate_phase("incremental_history");
        }

        let mut chunks           = Vec::new();
        let mut page_token       = None::<String>;
        let mut new_history_id   = history_id.clone();
        let mut messages_left    = self.max_messages.map(|m| m as usize);
        let mut stopped_early    = false;

        loop {
            let history = client
                .list_history(&history_id, page_token.as_deref())
                .await
                .map_err(|e| ConnectorError::Api(e.to_string()))?;

            if let Some(hid) = &history.history_id {
                new_history_id = hid.clone();
            }

            let added_ids_full: Vec<String> = history.history.iter()
                .flat_map(|record| &record.messages_added)
                .map(|added| added.message.id.clone())
                .collect();
            let full_page_len = added_ids_full.len();
            let mut added_ids = added_ids_full;

            if let Some(left) = messages_left.as_mut() {
                if *left == 0 {
                    stopped_early = true;
                    break;
                }
                if added_ids.len() > *left {
                    added_ids.truncate(*left);
                }
            }

            let dropped_on_page = full_page_len > added_ids.len();

            let n = added_ids.len().max(1);

            for (j, id) in added_ids.iter().enumerate() {
                if let Ok(msg) = client.get_message(id).await {
                    if let Some(chunk) = normalize_message(&msg, chunks.len()) {
                        chunks.push(chunk);
                    }
                }
                if let Some(p) = &self.progress {
                    let done = j + 1;
                    if done % 10 == 0 || done == added_ids.len() {
                        p.set_message_list_progress(done, n, chunks.len());
                    }
                }
            }

            if let Some(left) = messages_left.as_mut() {
                *left = left.saturating_sub(added_ids.len());
            }

            let has_more = history.next_page_token.is_some();
            if dropped_on_page || (messages_left == Some(0) && has_more) {
                stopped_early = true;
                break;
            }

            match history.next_page_token {
                Some(pt) => page_token = Some(pt),
                None     => break,
            }
        }

        if stopped_early {
            println!(
                "[gmail] Incremental sync hit max_messages cap or empty budget with more history pages — historyId left unchanged; run sync again to continue."
            );
            println!("[gmail] Incremental sync (partial): {} new chunks", chunks.len());
            return Ok(chunks);
        }

        let mut state = SyncState::load().unwrap_or_default();
        state.history_id   = Some(new_history_id);
        state.total_synced += chunks.len();
        let _ = state.save();

        println!("[gmail] Incremental sync: {} new chunks", chunks.len());
        Ok(chunks)
    }

    /// Query sync: fetch messages matching a Gmail search query.
    async fn sync_query(
        &self,
        client: &mut GmailClient,
        query: &str,
    ) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        println!("[gmail] Query sync: '{query}'");

        if let Some(p) = &self.progress {
            p.set_indeterminate_phase("query_list_messages");
        }

        let msg_refs = client
            .list_messages(Some(query), self.max_messages)
            .await
            .map_err(|e| ConnectorError::Api(e.to_string()))?;

        if let Some(p) = &self.progress {
            p.set_thread_totals(msg_refs.len());
        }

        let mut chunks = Vec::new();
        let total = msg_refs.len().max(1);

        for (i, msg_ref) in msg_refs.iter().enumerate() {
            if let Ok(msg) = client.get_message(&msg_ref.id).await {
                if let Some(chunk) = normalize_message(&msg, chunks.len()) {
                    chunks.push(chunk);
                }
            }
            if let Some(p) = &self.progress {
                let done = i + 1;
                if done % 15 == 0 || done == msg_refs.len() {
                    p.set_message_list_progress(done, total, chunks.len());
                }
            }
        }

        println!("[gmail] Query sync complete: {} chunks", chunks.len());
        Ok(chunks)
    }
}

impl Default for GmailConnector {
    fn default() -> Self { Self::new() }
}

// ── FluvioConnector impl ──────────────────────────────────────────────────────

impl FluvioConnector for GmailConnector {
    fn domain(&self) -> Domain {
        Domain::Email
    }

    fn name(&self) -> &str {
        "gmail"
    }

    /// source controls sync mode:
    ///   "full"        → fetch everything
    ///   "incremental" → fetch since last historyId
    ///   anything else → treat as Gmail search query
    fn extract(&self, source: &str) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        self.check_auth()?;

        let mode = SyncMode::from_source(source);

        // FluvioConnector::extract is sync but our Gmail client is async.
        // We create a local Tokio runtime here so the connector works from
        // both async (server) and sync (CLI) contexts.
        let rt = tokio::runtime::Handle::try_current();

        match rt {
            // Already inside a Tokio runtime (server context) — use block_in_place.
            Ok(handle) => {
                tokio::task::block_in_place(|| {
                    handle.block_on(self.run(mode))
                })
            }
            // No runtime (CLI context) — create one.
            Err(_) => {
                tokio::runtime::Runtime::new()
                    .map_err(|e| ConnectorError::Api(e.to_string()))?
                    .block_on(self.run(mode))
            }
        }
    }
}

impl GmailConnector {
    async fn run(&self, mode: SyncMode) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        let mut client = GmailClient::new().await
            .map_err(|e| ConnectorError::Auth(e.to_string()))?;

        match mode {
            SyncMode::Full          => self.sync_full(&mut client).await,
            SyncMode::Incremental   => self.sync_incremental(&mut client).await,
            SyncMode::Query(q)      => self.sync_query(&mut client, &q).await,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_mode_from_source_full() {
        assert!(matches!(SyncMode::from_source("full"), SyncMode::Full));
    }

    #[test]
    fn test_sync_mode_from_source_incremental() {
        assert!(matches!(SyncMode::from_source("incremental"), SyncMode::Incremental));
    }

    #[test]
    fn test_sync_mode_from_source_query() {
        let mode = SyncMode::from_source("newer_than:7d");
        assert!(matches!(mode, SyncMode::Query(_)));
        if let SyncMode::Query(q) = mode {
            assert_eq!(q, "newer_than:7d");
        }
    }

    #[test]
    fn test_sync_mode_whitespace_trimmed() {
        assert!(matches!(SyncMode::from_source("  full  "), SyncMode::Full));
    }

    #[test]
    fn test_connector_domain() {
        let c = GmailConnector::new();
        assert_eq!(c.domain(), Domain::Email);
    }

    #[test]
    fn test_connector_name() {
        let c = GmailConnector::new();
        assert_eq!(c.name(), "gmail");
    }

    #[test]
    fn test_check_auth_fails_without_credentials() {
        // Only fails if ~/.fluvio/credentials/gmail.json doesn't exist.
        // In CI this always fails — locally may pass.
        let c = GmailConnector::new();
        if !credentials_exist() {
            assert!(c.check_auth().is_err());
        }
    }

    #[test]
    fn test_connector_with_max_messages() {
        let c = GmailConnector::new().with_max_messages(100);
        assert_eq!(c.max_threads, Some(100));
        assert_eq!(c.max_messages, Some(100));
    }

    /// Integration test — only runs with `cargo test -- --ignored`
    /// Requires ~/.fluvio/credentials/gmail.json and real Gmail access.
    #[test]
    #[ignore]
    fn integration_extract_recent() {
        let connector = GmailConnector::new().with_max_messages(5);
        let chunks = connector.extract("newer_than:7d").unwrap();
        assert!(!chunks.is_empty());
        println!("Got {} chunks", chunks.len());
        for chunk in chunks.iter().take(3) {
            println!(
                "  [{}] {}",
                chunk.source_uri,
                chunk.text.chars().take(80).collect::<String>()
            );
        }
    }
}