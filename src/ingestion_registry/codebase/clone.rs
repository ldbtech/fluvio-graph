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

use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

// ── Errors ────────────────────────────────────────────────────────────────────
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
    #[error("repository not cloned: {0}")]
    NotCloned(String),
}

/// Repo Identity --------------------------------------------------------------
/// 
///! Parsed Github Repo Coordinates.
#[derive(Debug, Clone)]
pub struct RepoRef {
    pub owner: String,
    pub repo: String,
    pub branch: Option<String>,
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
    pub fn parse(url: &str) -> Result<Self, CloneError> {
        let url = url.trim().trim_end_matches('/');

        if url.is_empty() {
            return Err(CloneError::InvalidUrl("empty URL".to_string()));
        }

        // normalize: Strip everything up to and including "github.com/" 
        // So we can always work with "owner/repo[/tree/branch]";
        let path = if let Some(pos) = url.find("github.com/") {
            &url[pos + "github.com/".len()..]
        } else { 
            // Already in the desired format.
            url
        };

        // Filter empty segments so trailing slashes don't create empty parts.
        let parts: Vec<&str> = path
                    .split('/')
                    .filter(|s| !s.is_empty())
                    .collect();
        if parts.len() < 2 {
            return Err(CloneError::InvalidUrl(format!("Expected owner/repo, got {}: ", url)));
        }

        let owner = parts[0].to_string();
        let repo = parts[1]
               .trim_end_matches(".git")
               .to_string();

        let branch: Option<String> = if parts.len() >= 4 && parts[2] == "tree" {
            Some(parts[3].to_string())
        } else {
            None
        };

        if owner.is_empty() || repo.is_empty() {
            return Err(CloneError::InvalidUrl(format!("owner or repo is empty in {}: ", url)));
        }

        Ok(Self {
            owner,
            repo,
            branch
        })
    }

    // Canonical clone URL.
    pub fn clone_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }

    // Unique key for this repo - used as directory name segment.
    pub fn key(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
    
}

/// --- Path --------------------------------------------------------------------
/// 
/// ~/.fluvio/repos/
///!
pub fn repos_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(p) = test_repos_override::get() {
        return p;
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE")) // windows fallback.
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".fluvio").join("repos")
}

/// When set on the current thread (via [`TestReposRootGuard`]), [`repos_dir`] returns that path
/// instead of `~/.fluvio/repos`. Used by unit tests so they stay writable in sandboxes and
/// parallel-safe without touching the real home directory.
#[cfg(test)]
mod test_repos_override {
    use std::cell::RefCell;
    use std::path::PathBuf;

    thread_local! {
        static OVERRIDE: RefCell<Option<PathBuf>> = RefCell::new(None);
    }

    pub fn get() -> Option<PathBuf> {
        OVERRIDE.with(|o| o.borrow().clone())
    }

    fn set(p: Option<PathBuf>) {
        OVERRIDE.with(|o| *o.borrow_mut() = p);
    }

    pub struct Guard;

    impl Guard {
        pub fn new(root: PathBuf) -> Self {
            set(Some(root));
            Self
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            set(None);
        }
    }
}

#[cfg(test)]
pub use test_repos_override::Guard as TestReposRootGuard;

/// ~/.fluvio/repos/{owner}/{repo}/
///!
pub fn repo_path(repo: &RepoRef) -> PathBuf {
    repos_dir().join(&repo.owner).join(&repo.repo)
}

/// Returns true if the repo has already been cloned. 
pub fn is_cloned(repo: &RepoRef) -> bool {
    repo_path(repo).exists()
}

/// Relative file paths under a clone (skips `.git/`), for UI file trees.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ListFilesResult {
    pub paths: Vec<String>,
    pub truncated: bool,
}

const LIST_MAX_FILES: usize = 8_000;
const LIST_MAX_DEPTH: usize = 32;

/// Lists text-relevant files under `~/.fluvio/repos/{owner}/{repo}/` after a successful clone.
///
/// `url` is parsed with [`RepoRef::parse`] the same way as [`clone_or_pull`].
pub fn list_cloned_file_paths(url: &str) -> Result<ListFilesResult, CloneError> {
    let repo = RepoRef::parse(url)?;
    let root = repo_path(&repo);
    if !root.is_dir() {
        return Err(CloneError::NotCloned(format!(
            "expected clone directory {}",
            root.display()
        )));
    }
    let mut paths = Vec::new();
    let mut truncated = false;
    collect_repo_files(
        &root,
        &root,
        0,
        LIST_MAX_DEPTH,
        &mut paths,
        LIST_MAX_FILES,
        &mut truncated,
    )?;
    paths.sort();
    Ok(ListFilesResult { paths, truncated })
}

fn collect_repo_files(
    base: &Path,
    dir: &Path,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<String>,
    cap: usize,
    truncated: &mut bool,
) -> Result<(), CloneError> {
    if depth > max_depth {
        return Ok(());
    }
    if out.len() >= cap {
        *truncated = true;
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(std::result::Result::ok).collect();
    entries.sort_by_key(|e| e.file_name());

    for ent in entries {
        if out.len() >= cap {
            *truncated = true;
            return Ok(());
        }
        let name = ent.file_name();
        if name == ".git" {
            continue;
        }
        let path = ent.path();
        let rel = path
            .strip_prefix(base)
            .map_err(|_| CloneError::InvalidUrl("path prefix mismatch".to_string()))?;
        let rel_s = rel.to_string_lossy().replace('\\', "/");
        let meta = ent.metadata()?;
        if meta.is_dir() {
            collect_repo_files(base, &path, depth + 1, max_depth, out, cap, truncated)?;
        } else if meta.is_file() {
            out.push(rel_s);
        }
    }
    Ok(())
}

/// Clone result ----------------------------------------------------------------
/// 
///! The result of a git clone operation.
#[derive(Debug, Clone)]
pub struct CloneResult {
    pub owner: String,
    pub repo: String,
    pub local_path: PathBuf,

    /// true = fresh clone, false = already existed (pulled latest)
    pub was_cloned: bool,
}

///
///! Public API ----------------------------------------------------------------
/// 
///! Clone a repository.
pub fn clone_or_pull(url: &str) -> Result<CloneResult, CloneError> {
    // Verify git exists.
    if !std::path::Path::new("/usr/bin/git").exists() {
        return Err(CloneError::GitNotFound);
    }

    let r = RepoRef::parse(url)?;

    if is_cloned(&r) {
        pull(&r)?;
        return Ok(CloneResult {
            owner: r.owner.clone(),
            repo: r.repo.clone(),
            local_path: repo_path(&r),
            was_cloned: false,
        });
    }

    clone(&r)?;

    Ok(CloneResult {
        owner: r.owner.clone(),
        repo: r.repo.clone(),
        local_path: repo_path(&r),
        was_cloned: true,
    })
}

/// Delete a cloned repository from disk.
pub fn delete_repo(repo: &RepoRef) -> Result<(), CloneError> {
    let path = repo_path(repo);
    if path.exists(){
        std::fs::remove_dir_all(&path)?;
    }
    Ok(())
}

/// Internal helpers ------------------------------------------------------------
///! 
/// Clone a repository.
fn clone(repo: &RepoRef) -> Result<(), CloneError> {
    let destination = repo_path(repo);
    std::fs::create_dir_all(&destination)?;

    let mut cmd = Command::new("/usr/bin/git");
    cmd.arg("clone")
        .arg("--depth=1") // shallow clone - we dont need history.
        .arg("--single-branch"); // only the default (or specific) branch.
    
    if let Some(branch) = &repo.branch {
        cmd.arg("--branch").arg(branch);
    }

    cmd.arg(repo.clone_url())
        .arg(&destination);

    let output = cmd.output()?;

    if !output.status.success() {
        let _ = std::fs::remove_dir_all(&destination);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CloneError::CloneFailed(format!("git clone failed: {}", stderr)));
    }

    Ok(())
}

/// Pull a repository.
/// 
///! Pulls the latest changes from the remote repository.
fn pull(repo: &RepoRef) -> Result<(), CloneError> {
    let path = repo_path(repo);

    let output = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&path)
        .arg("pull")
        .arg("--ff-only")   // fast-forward only — safe for shallow clones
        .output()?;

    if !output.status.success() {
    
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CloneError::PullFailed(format!("git pull failed: {}", stderr)));
    }

    Ok(())
}

/// Testing Helpers -------------------------------------------------------------
///!
/// 
// ── Tests ─────────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
 
    // ── RepoRef::parse ────────────────────────────────────────────────────────
 
    #[test]
    fn test_parse_full_url() {
        let r = RepoRef::parse("https://github.com/pjasicek/OpenClaw").unwrap();
        assert_eq!(r.owner, "pjasicek");
        assert_eq!(r.repo,  "OpenClaw");
        assert!(r.branch.is_none());
    }
 
    #[test]
    fn test_parse_with_git_suffix() {
        let r = RepoRef::parse("https://github.com/owner/repo.git").unwrap();
        assert_eq!(r.repo, "repo");
    }
 
    #[test]
    fn test_parse_with_branch() {
        let r = RepoRef::parse("https://github.com/owner/repo/tree/develop").unwrap();
        assert_eq!(r.owner,  "owner");
        assert_eq!(r.repo,   "repo");
        assert_eq!(r.branch, Some("develop".to_string()));
    }
 
    #[test]
    fn test_parse_short_form() {
        let r = RepoRef::parse("owner/repo").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo,  "repo");
    }
 
    #[test]
    fn test_parse_without_protocol() {
        let r = RepoRef::parse("github.com/owner/repo").unwrap();
        assert_eq!(r.owner, "owner");
        assert_eq!(r.repo,  "repo");
    }
 
    #[test]
    fn test_parse_trailing_slash() {
        let r = RepoRef::parse("https://github.com/owner/repo/").unwrap();
        assert_eq!(r.repo, "repo");
    }
 
    #[test]
    fn test_parse_invalid_no_repo() {
        let result = RepoRef::parse("https://github.com/owner");
        assert!(matches!(result, Err(CloneError::InvalidUrl(_))));
    }
 
    #[test]
    fn test_parse_empty() {
        let result = RepoRef::parse("");
        assert!(matches!(result, Err(CloneError::InvalidUrl(_))));
    }
 
    #[test]
    fn test_clone_url() {
        let r = RepoRef::parse("https://github.com/owner/repo").unwrap();
        assert_eq!(r.clone_url(), "https://github.com/owner/repo.git");
    }
 
    #[test]
    fn test_key() {
        let r = RepoRef::parse("https://github.com/owner/repo").unwrap();
        assert_eq!(r.key(), "owner/repo");
    }
 
    #[test]
    fn test_repo_path() {
        let r = RepoRef::parse("https://github.com/owner/myrepo").unwrap();
        let path = repo_path(&r);
        assert!(path.ends_with("owner/myrepo"));
        assert!(path.to_string_lossy().contains(".fluvio/repos"));
    }
 
    #[test]
    fn test_is_cloned_false_for_new_repo() {
        let r = RepoRef::parse("https://github.com/fake/doesnotexist999").unwrap();
        assert!(!is_cloned(&r));
    }
 
    /// Integration test — actually clones a small public repo.
    /// Only runs with `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn integration_clone_small_repo() {
        // miniserve is tiny (~2MB) and public.
        let result = clone_or_pull("https://github.com/svenstaro/miniserve").unwrap();
        assert_eq!(result.owner, "svenstaro");
        assert_eq!(result.repo,  "miniserve");
        assert!(result.local_path.exists());
        assert!(result.local_path.join(".git").exists());
        assert!(result.was_cloned);
 
        // Second call should pull, not clone.
        let result2 = clone_or_pull("https://github.com/svenstaro/miniserve").unwrap();
        assert!(!result2.was_cloned);
 
        // Clean up.
        let r = RepoRef::parse("https://github.com/svenstaro/miniserve").unwrap();
        delete_repo(&r).unwrap();
        assert!(!is_cloned(&r));
    }
}
  