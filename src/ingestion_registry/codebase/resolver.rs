///! resolver.rs
///!
///! Given a single source file in a cloned repo, resolves its import graph.
///! recursively - pulling in every internal dependency up to a configurable depth.
///!
///! This is what powers "click a file -> graph grows outward along imports".
///!
/// 
///! Exmaple:
///! User clicks `src/server.rs`
///!  -> Resolver finds: imports ingestion_registery, graph, query.
///!  -> each of those imports files
///!  -> result: a subgraph centered on server.rs showing its full dependency slice.
///!
///! External Imports: (std, crate.io) are ignoed only files that actually.
///! Exists in the repo are followed.
///!
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::ingestion_registry::connector::NormalizedChunk;
use crate::ingestion_registry::connector::ConnectorError;

use super::clone::{repo_path,RepoRef, CloneError};
use super::normalizer::normalize_file;
use super::parser::{parse_file, Language};
use super::tree::Language as TreeLanguage;

// ── Errors Mapping ────────────────────────────────────────────────────────────────────
fn map_clone_error(e: CloneError) -> ConnectorError {
    match e {
        CloneError::InvalidUrl(m)               => ConnectorError::Parse(m),
        CloneError::GitNotFound                         => ConnectorError::NotConfigured("git not found".into()),
        CloneError::CloneFailed(m)
        | CloneError::PullFailed(m)
        | CloneError::PushFailed(m)             => ConnectorError::Api(m),
        CloneError::Io(e)                        => ConnectorError::Io(e),
        CloneError::NotCloned(m)                => ConnectorError::NotConfigured(m),
    }
}

// ── Resolution Result ─────────────────────────────────────────────────────────────────
// The full result of resolving a file and its dependency.
#[derive(Debug)]
pub struct ResolvedGraph {
    // All chunks produced - file + symbols chunks for every resolve file.
    pub chunks: Vec<NormalizedChunk>,
    // Relatives path of every file that was resolved (including the root)
    pub resolved_paths: Vec<String>,
    // Paths that were referenced in imports but not found on the desk. (external or missing)
    pub unresolved_imports: Vec<String>,
    // How deep the resoluton went - (0 = root file only)
    pub max_depth_reached: usize,
}

/// --- Public API -------------------------------------------------------------------------

/// Resolsve a single file and its transitions imports dependecies. 
/// `url` is the GitHub repo URL or owner/repo 
/// `rel_path` - repo-relative path to the entry file e.g. "src/server.rs"
/// `max_depth` - how many imports hops to follow (1 = direct imports only, 2 = import of imports, etc.)
/// `max_files` - hard cap on total files to resolve (prevents explosion on large repos)
/// 
pub fn resolve_file(
    url: &str, 
    rel_path: &str, 
    max_depth: usize, 
    max_files: usize) 
    -> Result<ResolvedGraph, ConnectorError> {
        let r = RepoRef::parse(url).map_err(map_clone_error)?;

        let root = repo_path(&r);

        if !root.join(".git").exists(){
            return Err(ConnectorError::NotConfigured(format!("repo not cloned: {}", r.key())));
        }

        let mut visited:                    HashSet<String>      = HashSet::new();
        let mut unresolved_imports:         Vec<String>          = Vec::new();
        let mut all_chunks:                 Vec<NormalizedChunk> = Vec::new();
        let mut resolved_paths:             Vec<String>          = Vec::new();
        let mut max_depth_reached:          usize                = 0;
        let mut chunk_index:                usize                = 0;

        // BFS QUEUE (relative_path, depth)
        let mut queue:                      VecDeque<(String, usize)> = VecDeque::new();

        queue.push_back((normalize_path(rel_path), 0));

        while let Some((current_rel, depth)) = queue.pop_front(){
            // Bounds checks
            if visited.len() >= max_files {
                break;
            }

            if visited.contains(&current_rel) {
                continue;
            }

            visited.insert(current_rel.clone());

            let abs_path = root.join(&current_rel);

            if !abs_path.exists() {
                unresolved_imports.push(current_rel.clone());
                continue;
            }

            // Detect language
            let ext = abs_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default();
            let language = TreeLanguage::from_extension(&ext);
            if !language.is_source() {
                continue;
            }

            // Perse a file
            let parsed = match parse_file(&abs_path, &root, &language) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[resolver] skipping {}: {}", current_rel, e);
                    continue;
                }
            };

            // Track resolved. 
            resolved_paths.push(current_rel.clone());
            if depth > max_depth_reached {
                max_depth_reached = depth;
            }

            // Normalize to chunks
            let file_chunks = normalize_file(&parsed, &r.owner, &r.repo, chunk_index);
            chunk_index += file_chunks.len();
            all_chunks.extend(file_chunks);

            // Enqueue resolved imports for next depth level.
            if depth < max_depth {
                for import in &parsed.imports {
                    if let Some(resolved_path) = &import.resolved {
                        let norm = normalize_path(resolved_path);
                        if !visited.contains(&norm) {
                            // Resolve relative path (e.g "../utils") against current file's dir.
                            let resolved = resolved_relative(&current_rel, &norm, &root);
                            if let Some(abs) = resolved {
                                let rel = abs
                                     .strip_prefix(&root)
                                     .map(|p| p.to_string_lossy().replace("\\", "/"))
                                     .unwrap_or_default();
                                queue.push_back((rel, depth + 1));
                            }else {
                                queue.push_back((norm, depth + 1));
                            }
                        }
                    } else {
                        // External import - track as unresolved for info
                        unresolved_imports.push(import.raw.clone());
                    }
                }
            }
        
        }

        Ok(ResolvedGraph {
            chunks: all_chunks,
            resolved_paths,
            unresolved_imports,
            max_depth_reached,
        })


    }

// ── Helpers ──────────────────────────────────────────────────────────────────────
fn normalize_path(path: &str) -> String {
    path.replace("\\", "/").to_lowercase()
}

// Resolve a potentiall relative import both against the current file's directory 
/// e.g: current: = "src/ingestion/email/connector.rs", import = "../auth/oauth.rs"
/// -> "src/ingestion/auth/oauth.rs"
fn resolved_relative(current_rel: &str, import_path: &str, root: &Path) -> Option<PathBuf> {
    if !import_path.starts_with("..") {
        // Not a parent-relative path — try as repo-relative directly.
        let candidate = root.join(import_path);
        return if candidate.exists() { Some(candidate) } else { None };
    }
 
    // Resolve `..` components relative to current file's directory.
    let current_dir = Path::new(current_rel).parent()?;
    let candidate   = current_dir.join(import_path);
 
    // Normalize the path (collapse `..` components).
    let normalized = normalize_dotdot(&candidate);
    let abs = root.join(&normalized);
 
    if abs.exists() { Some(abs) } else { None }
}

/// Collapse `..` components in a relative path without touching the filesystem.
fn normalize_dotdot(path: &Path) -> PathBuf {
    let mut components: Vec<std::ffi::OsString> = Vec::new();

    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => {
                components.push(other.as_os_str().to_os_string());
            }
        }
    }

    components.iter().collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_registry::codebase::clone::{repo_path, RepoRef, TestReposRootGuard};
    use std::fs;
    use uuid::Uuid;
 
    fn make_test_repo() -> (PathBuf, RepoRef, TestReposRootGuard) {
        let id    = Uuid::new_v4();
        let owner = format!("res-o-{id}");
        let repo  = format!("res-r-{id}");
        let r     = RepoRef::parse(&format!("{owner}/{repo}")).unwrap();
 
        let repos_root = std::env::temp_dir().join(format!("fluvio_resolver_repos_{id}"));
        fs::create_dir_all(&repos_root).unwrap();
        let guard = TestReposRootGuard::new(repos_root);
 
        let dest = repo_path(&r);
        fs::create_dir_all(dest.join("src/auth")).unwrap();
        fs::create_dir_all(dest.join(".git")).unwrap(); // mark as cloned
 
        // Entry file — imports two internal files.
        fs::write(dest.join("src/server.rs"), r#"
            use crate::auth::oauth;
            use crate::graph::Graph;
            use std::sync::Arc;
            
            pub fn serve() {}
            pub struct Server {}
            "#).unwrap();
 
        // Direct dependency.
        fs::write(dest.join("src/auth/oauth.rs"), r#"
            use crate::auth::token_store;
            
            pub fn get_auth_url() -> String { String::new() }
            pub struct OAuthState {}
            "#).unwrap();
 
        // Second-level dependency.
        fs::write(dest.join("src/auth/token_store.rs"), r#"
                use std::path::PathBuf;
                
                pub fn save_token() {}
                pub struct Token {}
                "#).unwrap();
 
        // External-only file (no internal imports).
        fs::write(dest.join("src/graph.rs"), r#"
                use std::collections::HashMap;
                
                pub struct Graph {}
                impl Graph { pub fn new() -> Self { Self {} } }
                "#).unwrap();
 
        (dest, r, guard)
    }
 
    fn url(r: &RepoRef) -> String { format!("{}/{}", r.owner, r.repo) }
 
    #[test]
    fn test_resolve_single_file_no_deps() {
        let (dest, r, _guard) = make_test_repo();
 
        let result = resolve_file(&url(&r), "src/graph.rs", 2, 50).unwrap();
 
        assert_eq!(result.resolved_paths, vec!["src/graph.rs"]);
        assert_eq!(result.max_depth_reached, 0);
        assert!(!result.chunks.is_empty());
 
        let _ = fs::remove_dir_all(&dest);
    }
 
    #[test]
    fn test_resolve_depth_0_only_root() {
        let (dest, r, _guard) = make_test_repo();
 
        let result = resolve_file(&url(&r), "src/server.rs", 0, 50).unwrap();
 
        // depth=0 means root file only, no imports followed.
        assert_eq!(result.resolved_paths.len(), 1);
        assert!(result.resolved_paths.contains(&"src/server.rs".to_string()));
        assert_eq!(result.max_depth_reached, 0);
 
        let _ = fs::remove_dir_all(&dest);
    }
 
    #[test]
    fn test_resolve_depth_1_direct_imports() {
        let (dest, r, _guard) = make_test_repo();
 
        let result = resolve_file(&url(&r), "src/server.rs", 1, 50).unwrap();
 
        // Should include server.rs + its direct internal imports.
        assert!(result.resolved_paths.contains(&"src/server.rs".to_string()));
        // graph.rs is a direct import.
        assert!(result.resolved_paths.contains(&"src/graph.rs".to_string()));
        assert_eq!(result.max_depth_reached, 1);
 
        let _ = fs::remove_dir_all(&dest);
    }
 
    #[test]
    fn test_resolve_max_files_cap() {
        let (dest, r, _guard) = make_test_repo();
 
        // Cap at 1 file — should only get the root.
        let result = resolve_file(&url(&r), "src/server.rs", 5, 1).unwrap();
        assert_eq!(result.resolved_paths.len(), 1);
 
        let _ = fs::remove_dir_all(&dest);
    }
 
    #[test]
    fn test_resolve_no_duplicate_files() {
        let (dest, r, _guard) = make_test_repo();
 
        let result = resolve_file(&url(&r), "src/server.rs", 3, 50).unwrap();
 
        // No file should appear twice.
        let mut seen = HashSet::new();
        for path in &result.resolved_paths {
            assert!(seen.insert(path.clone()), "duplicate path: {path}");
        }
 
        let _ = fs::remove_dir_all(&dest);
    }
 
    #[test]
    fn test_resolve_chunks_all_codebase_domain() {
        let (dest, r, _guard) = make_test_repo();
 
        let result = resolve_file(&url(&r), "src/server.rs", 2, 50).unwrap();
 
        use crate::graph::enums::Domain;
        assert!(result.chunks.iter().all(|c| c.domain == Domain::Codebase));
        assert!(result.chunks.iter().all(|c| {
            c.metadata.get("source").map(|s| s == "codebase").unwrap_or(false)
        }));
 
        let _ = fs::remove_dir_all(&dest);
    }
 
    #[test]
    fn test_resolve_not_cloned_error() {
        let result = resolve_file(
            "fake/repo-xyz-does-not-exist",
            "src/main.rs",
            2,
            50,
        );
        assert!(matches!(result, Err(ConnectorError::NotConfigured(_))));
    }
 
    #[test]
    fn test_normalize_dotdot() {
        let p = Path::new("src/auth/../graph.rs");
        let n = normalize_dotdot(p);
        assert_eq!(n, PathBuf::from("src/graph.rs"));
    }
 
    #[test]
    fn test_normalize_dotdot_multiple() {
        let p = Path::new("src/auth/oauth/../../server.rs");
        let n = normalize_dotdot(p);
        assert_eq!(n, PathBuf::from("src/server.rs"));
    }
 
    #[test]
    fn test_chunk_indices_sequential() {
        let (dest, r, _guard) = make_test_repo();
 
        let result = resolve_file(&url(&r), "src/server.rs", 2, 50).unwrap();
 
        let indices: Vec<usize> = result.chunks.iter().map(|c| c.chunk_index).collect();
        for w in indices.windows(2) {
            assert!(w[1] > w[0], "chunk indices not sequential");
        }
 
        let _ = fs::remove_dir_all(&dest);
    }
}