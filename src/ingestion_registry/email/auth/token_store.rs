use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use chrono::{Utc};
use thiserror::Error;

// ── Errors ────────────────────────────────────────────────────────────────────
#[derive(Debug, Error)]
pub enum TokenStoreError {
    #[error("credentials not found at {0} — complete Gmail OAuth on kg-engine first (POST /connect/gmail/start while signed in, or web UI Sources → Gmail)")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
 
// ── Token ─────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl GmailToken { // return true if token is expired already or within 60 seconds.
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.expires_at <= now + 60
    }
}

// --- Path to token file ──────────────────────────────────────────────────────

// ~/.fluvio/
pub fn fluvio_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE")) // windows fallback.
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".fluvio")
}

// ~/.fluvio/credentials/
pub fn credentials_dir() -> PathBuf {
    fluvio_dir().join("credentials")
}

// ~/.fluvio/credentials/gmail.json
pub fn gmail_token_path() -> PathBuf {
    credentials_dir().join("gmail.json")
}

// --- READ / Write ----------------------------------------------------------------
pub fn load_token() -> Result<GmailToken, TokenStoreError> {
    let path = gmail_token_path();
    if !path.exists() {
        return Err(TokenStoreError::NotFound(path.display().to_string()));
    }
    let json = std::fs::read_to_string(&path)?;
    let token: GmailToken = serde_json::from_str(&json)?;
    Ok(token)
}

// presistent a gmail token to disk. 
// Create ~/.fluvio/credentials/ if it does not exists.
pub fn save_token(token: &GmailToken) -> Result<(), TokenStoreError> {
    let dir = credentials_dir();
    std::fs::create_dir_all(&dir)?;
    let path = gmail_token_path();
    let json = serde_json::to_string(token)?;
    std::fs::write(&path, json)?;
    Ok(())    
}

// Delete stored credentials (used by `fluvio disconnect gmail`)
pub fn delete_token() -> Result<(), TokenStoreError> {
    let path = gmail_token_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn credentials_exist() -> bool {
    gmail_token_path().exists()
}

//// TESTS ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;
    use std::sync::Mutex;

    static HOME_ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Point `HOME` at a temp dir so tests don't touch a real `~/.fluvio/`.
    /// Uses `target/` under the crate (writable in CI sandboxes that block system temp).
    fn with_temp_home<F: FnOnce()>(f: F) {
        let _guard = HOME_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let unique = format!("home-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let temp_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("fluvio-gmail-test-homes")
            .join(unique);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let prev_home = env::var("HOME").ok();
        // SAFETY: `set_var` is unsafe in Rust 2024 unless no other thread reads the environment concurrently.
        // `HOME_ENV_LOCK` serializes all `with_temp_home` runs in this crate.
        unsafe {
            env::set_var("HOME", temp_dir.as_os_str());
        }
        f();
        // SAFETY: same contract as `set_var` above.
        unsafe {
            match &prev_home {
                Some(h) => env::set_var("HOME", h),
                None => env::remove_var("HOME"),
            }
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_save_and_load_token(){
        with_temp_home(|| {
            let token = GmailToken {
                access_token: "test_access_token".to_string(),
                refresh_token: "test_refresh_token".to_string(),
                expires_at: Utc::now().timestamp() + 3600,
            };
            save_token(&token).unwrap();
            let loaded = load_token().unwrap();
            assert_eq!(loaded.access_token, token.access_token);
            assert_eq!(loaded.refresh_token, token.refresh_token);
            assert!(!loaded.is_expired());
        });
    }

    #[test]
    fn test_not_found_when_missing() {
        with_temp_home(|| {
            let result = load_token();
            assert!(matches!(result, Err(TokenStoreError::NotFound(_))));
        });
    }

    #[test]
    fn test_delete_token() {
        with_temp_home(|| {
            let token = GmailToken {
                access_token: "test_access_token".to_string(),
                refresh_token: "test_refresh_token".to_string(),
                expires_at: Utc::now().timestamp() + 3600,
            };
            save_token(&token).unwrap();
            assert!(credentials_exist());
            delete_token().unwrap();
            assert!(!credentials_exist());
        });
    }

    #[test]
    fn test_is_expired() {
        let token = GmailToken {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            expires_at: Utc::now().timestamp() - 3600,
        };
        assert!(token.is_expired());

        let fresh = GmailToken {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            expires_at: Utc::now().timestamp() + 3600,
        };
        assert!(!fresh.is_expired());
    }
}