//! sync/state.rs
//!
//! SyncState — persists Gmail sync progress to ~/.fluvio/sync/gmail.json
//!
//! Stores the last Gmail historyId so incremental sync knows where to resume.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ingestion_registry::email::auth::token_store::fluvio_dir;

#[derive(Debug, Error)]
pub enum SyncStateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Persisted sync progress for Gmail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// The Gmail historyId after the last successful sync.
    /// None means no sync has been run yet → fall back to full sync.
    pub history_id: Option<String>,

    /// When the last sync completed.
    pub last_sync_at: Option<DateTime<Utc>>,

    /// Running total of chunks ingested across all syncs.
    pub total_synced: usize,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            history_id:   None,
            last_sync_at: None,
            total_synced: 0,
        }
    }
}

impl SyncState {
    fn path() -> std::path::PathBuf {
        fluvio_dir().join("sync").join("gmail.json")
    }

    /// Load from ~/.fluvio/sync/gmail.json — returns Default if not found.
    pub fn load() -> Result<Self, SyncStateError> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let json  = std::fs::read_to_string(path)?;
        let state = serde_json::from_str(&json)?;
        Ok(state)
    }

    /// Save to ~/.fluvio/sync/gmail.json — creates directory if needed.
    pub fn save(&mut self) -> Result<(), SyncStateError> {
        self.last_sync_at = Some(Utc::now());
        let dir  = fluvio_dir().join("sync");
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::path(), json)?;
        Ok(())
    }

    /// Reset — used when a full sync is forced.
    pub fn reset(&mut self) {
        self.history_id   = None;
        self.last_sync_at = None;
        self.total_synced = 0;
    }

    pub fn has_prior_sync(&self) -> bool {
        self.history_id.is_some()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_temp_home<F: FnOnce()>(f: F) {
        let tmp = std::env::temp_dir().join("fluvio_sync_test");
        std::fs::create_dir_all(&tmp).unwrap();
        unsafe {
            env::set_var("HOME", &tmp);
        }
        f();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_default_state() {
        let state = SyncState::default();
        assert!(state.history_id.is_none());
        assert!(state.last_sync_at.is_none());
        assert_eq!(state.total_synced, 0);
        assert!(!state.has_prior_sync());
    }

    #[test]
    fn test_save_and_load() {
        with_temp_home(|| {
            let mut state = SyncState {
                history_id:   Some("history_abc".to_string()),
                last_sync_at: None,
                total_synced: 42,
            };
            state.save().unwrap();

            let loaded = SyncState::load().unwrap();
            assert_eq!(loaded.history_id, Some("history_abc".to_string()));
            assert_eq!(loaded.total_synced, 42);
            assert!(loaded.last_sync_at.is_some()); // save() sets this
            assert!(loaded.has_prior_sync());
        });
    }

    #[test]
    fn test_load_returns_default_when_missing() {
        with_temp_home(|| {
            let state = SyncState::load().unwrap();
            assert!(state.history_id.is_none());
            assert_eq!(state.total_synced, 0);
        });
    }

    #[test]
    fn test_reset() {
        let mut state = SyncState {
            history_id:   Some("abc".to_string()),
            last_sync_at: Some(Utc::now()),
            total_synced: 100,
        };
        state.reset();
        assert!(!state.has_prior_sync());
        assert_eq!(state.total_synced, 0);
    }
}