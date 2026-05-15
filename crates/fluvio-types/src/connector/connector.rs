//! Connector types shared across fluvio-connectors and fluvio-collab.

use serde::{Deserialize, Serialize};

// ── ConnectorId ───────────────────────────────────────────────────────────────

/// The set of connectors Fluvio supports.
///
/// `Later` variants exist so the UI can render them as disabled
/// without special-casing in application logic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorId {
    /// GitHub — OAuth2, issues/PRs/commits/code search.
    GitHub,
    /// Gmail — OAuth2 (Google), threads/attachments/contacts.
    Gmail,
    /// Yahoo Finance — API key, quotes/news/financials.
    YahooFinance,
    /// Google Drive — OAuth2 (Google), docs/sheets as knowledge.
    GoogleDrive,
    /// Investment broker (Alpaca / IBKR) — guarded, OAuth2 + MFA.
    Broker,
}

impl std::fmt::Display for ConnectorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ConnectorId::GitHub       => "github",
            ConnectorId::Gmail        => "gmail",
            ConnectorId::YahooFinance => "yahoo_finance",
            ConnectorId::GoogleDrive  => "google_drive",
            ConnectorId::Broker       => "broker",
        };
        write!(f, "{s}")
    }
}

// ── ConnectorStatus ───────────────────────────────────────────────────────────

/// Whether a connector is available for use in this group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectorStatus {
    /// OAuth token present, connector is callable.
    Connected,
    /// OAuth initiated but token not yet received.
    Pending,
    /// Not yet connected — user must complete OAuth flow.
    Disconnected,
    /// Connector exists but is not yet available (shown as LATER in UI).
    ComingSoon,
}

// ── ConnectorConfig ───────────────────────────────────────────────────────────

/// Per-group connector configuration.
/// The actual OAuth tokens live in Postgres `connector_credentials`
/// inside `fluvio-connectors` — this struct is the public-facing config
/// visible to `fluvio-collab` and the UI (no secrets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub connector_id: ConnectorId,
    pub status:       ConnectorStatus,
    /// Connector-specific metadata (e.g. linked GitHub repo URL).
    pub meta:         std::collections::HashMap<String, String>,
}

impl ConnectorConfig {
    pub fn new(connector_id: ConnectorId) -> Self {
        Self {
            connector_id,
            status: ConnectorStatus::Disconnected,
            meta:   Default::default(),
        }
    }

    pub fn is_callable(&self) -> bool {
        self.status == ConnectorStatus::Connected
    }
}