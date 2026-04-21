//! cloner.rs
//!
//! Clones public GitHub repositories to ~/.fluvio/repos/{owner}/{repo}/
//!
//! Uses the system `git` binary — no external crates needed.
//!
//! Layout after clone:
//!   ~/.fluvio/
//!     repos/
//!       ldbtech/
//!         FluvioGraph/       ← git clone output lives here
//!       anthropics/
//!         anthropic-sdk-python/

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

//! ---- Error ----------------------------------------------------------------
//! 
#[derive(Debug, Error)]
pub enum CloneError {
    #[error("invalid GitHub URL: {0}")]
    InvalidUrl(String),
    #[error("git not found at /usr/bin/git")]
    GitNotFound,
    #[error("git clone failed: {0}")]
    CloneFailed(String),
    #[error("git pull failed: {0}")]
    PullFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git push failed: {0}")]
    PushFailed(String),
}

/// Repo Identity --------------------------------------------------------------
/// 
///! Parsed Github Repo Coordinates.
#[derive(Debug, Clone)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
    pub branch: String,
} 

impl RepoRef {
    /// Parse a github URL into owner + repo + optional branch.
    ///! 
    /// Accepts:
    ///   https://github.com/owner/repo
    ///   https://github.com/owner/repo.git
    ///   https://github.com/owner/repo/tree/main
    ///   github.com/owner/repo
    ///   owner/repo
    /// 
    
}

/// --- Path --------------------------------------------------------------------
/// 
/// ~/.fluvio/repos/
///!
pub fn repos_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE")) // windows fallback.
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".fluvio").join("repos")
}

/// ~/.fluvio/repos/{owner}/{repo}/
///!
pub fn repo_path(repo: &RepoRef) -> PathBuf {
    repos_dir().join(&repo.owner).join(&repo.repo)
}



