//! connector.rs
//!
//! CodebaseConnector — implements FluvioConnector for the codebase domain.
//!
//! `extract(url)` where url is a GitHub repo URL:
//!   1. Verifies the repo is already cloned (~/.fluvio/repos/owner/repo/)
//!   2. Walks the repo via tree.rs — finds all parseable source files
//!   3. Parses each file via parser.rs — extracts imports + symbols
//!   4. Normalizes via normalizer.rs — produces NormalizedChunk with pre-defined edges
//!   5. Returns the full Vec<NormalizedChunk> to the pipeline
//!
//! Clone is intentionally separate (`POST /codebase/clone` on kg-engine) so the user
//! controls when network I/O happens. `extract_*` is local disk work after clone.

use crate::graph::enums::Domain;
use crate::ingestion_registry::connector::{ConnectorError, FluvioConnector, NormalizedChunk};

use super::clone::{clone_or_pull, is_cloned, repo_path, CloneError, CloneResult, RepoRef};
use super::normalizer::normalize_file;
use super::parser::{parse_file, ParseError, enrich_with_calls};
use super::tree::{build_tree_from_path, flatten_files, Language, NodeKind};

// ── Error mapping ─────────────────────────────────────────────────────────────

fn map_clone_error(e: CloneError) -> ConnectorError {
    match e {
        CloneError::InvalidUrl(msg)   => ConnectorError::Parse(msg),
        CloneError::GitNotFound       => ConnectorError::NotConfigured(
            "system git not found at /usr/bin/git".to_string(),
        ),
        CloneError::CloneFailed(msg)
        | CloneError::PullFailed(msg)
        | CloneError::PushFailed(msg) => ConnectorError::Api(msg),
        CloneError::Io(err)           => ConnectorError::Io(err),
        CloneError::NotCloned(msg)    => ConnectorError::NotConfigured(msg),
    }
}

fn map_parse_error(e: ParseError) -> ConnectorError {
    match e {
        ParseError::Io { path, source } => ConnectorError::Io(source),
        ParseError::UnsupportedLanguage(lang) => ConnectorError::Parse(
            format!("unsupported language: {lang:?}"),
        ),
        ParseError::Regex(e) => ConnectorError::Parse(e.to_string()),
    }
}

// ── Connector ─────────────────────────────────────────────────────────────────

pub struct CodebaseConnector {
    /// Skip files larger than this (bytes). Default 512KB — avoids embedding
    /// generated files, lock files, minified JS, etc.
    pub max_file_bytes: u64,
}

fn normalize_rel_path(s: &str) -> String {
    s.replace('\\', "/")
}

/// True when `rel_path` is exactly `prefix` or under `prefix/` (repo-relative paths).
fn file_under_repo_prefix(rel_path: &str, prefix: &str) -> bool {
    let rel = normalize_rel_path(rel_path);
    let p = normalize_rel_path(prefix.trim());
    let p = p.trim_matches('/').to_string();
    if p.is_empty() {
        return true;
    }
    rel == p || rel.starts_with(&(p.clone() + "/"))
}

impl CodebaseConnector {
    pub fn new() -> Self {
        Self {
            max_file_bytes: 512 * 1024,
        }
    }

    pub fn with_max_file_bytes(mut self, bytes: u64) -> Self {
        self.max_file_bytes = bytes;
        self
    }

    /// Shallow clone or fast-forward pull of a public GitHub repo.
    pub fn clone_public_url(url: &str) -> Result<CloneResult, ConnectorError> {
        clone_or_pull(url).map_err(map_clone_error)
    }

    /// Parse and normalize a single file by its relative path within a cloned repo.
    /// Used by the server's /codebase/parse and /codebase/ingest endpoints.
    pub fn extract_file(
        url:      &str,
        rel_path: &str,
    ) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        let r    = RepoRef::parse(url).map_err(map_clone_error)?;
        let root = repo_path(&r);

        if !is_cloned(&r) {
            return Err(ConnectorError::NotConfigured(format!(
                "repo not cloned — run POST /codebase/clone (or POST /sync/codebase/clone) first: {}",
                r.key()
            )));
        }

        let abs_path = root.join(rel_path);
        let ext = abs_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let language = Language::from_extension(&ext);

        if !language.is_source() {
            return Err(ConnectorError::Parse(format!(
                "file is not a parseable source file: {rel_path}"
            )));
        }
        let mut parsed = parse_file(&abs_path, &root, &language)
                     .map_err(map_parse_error)?;
        if let Ok(source) = std::fs::read_to_string(&abs_path) {
            enrich_with_calls(&mut parsed, &source, &[]);
        }

        let chunks = normalize_file(&parsed, &r.owner, &r.repo, 0);
        Ok(chunks)
    }

    /// Parse + normalize only files whose repo-relative path lies under `path_prefix`
    /// (e.g. `"src/agent"` includes `src/agent/mod.rs`). Empty prefix is treated as full repo.
    pub fn extract_under_prefix(
        &self,
        source: &str,
        path_prefix: &str,
    ) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        let r = RepoRef::parse(source).map_err(map_clone_error)?;
        let root = repo_path(&r);

        if !is_cloned(&r) {
            return Err(ConnectorError::NotConfigured(format!(
                "repo '{}' is not cloned — call clone_public_url first",
                r.key()
            )));
        }

        let tree = build_tree_from_path(&root, &r.repo).map_err(|e| {
            ConnectorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;

        let all_files = flatten_files(&tree);
        let scoped: Vec<String> = all_files
            .into_iter()
            .filter(|rel| file_under_repo_prefix(rel, path_prefix))
            .collect();

        let mut all_chunks: Vec<NormalizedChunk> = Vec::new();
        let mut chunk_index = 0usize;
        let mut skipped = 0usize;
        let mut parsed_count = 0usize;

        for rel_path in &scoped {
            let abs_path = root.join(rel_path);

            if let Ok(meta) = std::fs::metadata(&abs_path) {
                if meta.len() > self.max_file_bytes {
                    skipped += 1;
                    continue;
                }
            }

            let ext = abs_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let language = Language::from_extension(&ext);

            if !language.is_source() {
                continue;
            }

            let mut parsed = match parse_file(&abs_path, &root, &language) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[codebase] skipping {rel_path}: {e}");
                    skipped += 1;
                    continue;
                }
            };
            if let Ok(source) = std::fs::read_to_string(&abs_path) {
                enrich_with_calls(&mut parsed, &source, &[]);
            }

            let file_chunks = normalize_file(&parsed, &r.owner, &r.repo, chunk_index);
            chunk_index += file_chunks.len();
            parsed_count += 1;
            all_chunks.extend(file_chunks);
        }

        println!(
            "[codebase] planet {:?}: {}/{} files parsed → {} chunks ({} skipped)",
            path_prefix,
            parsed_count,
            scoped.len(),
            all_chunks.len(),
            skipped,
        );

        Ok(all_chunks)
    }
}

impl Default for CodebaseConnector {
    fn default() -> Self {
        Self::new()
    }
}

// ── FluvioConnector impl ──────────────────────────────────────────────────────

impl FluvioConnector for CodebaseConnector {
    fn domain(&self) -> Domain {
        Domain::Codebase
    }

    fn name(&self) -> &str {
        "codebase"
    }

    /// Extract all parseable source files from a cloned repo.
    ///
    /// `source` is a GitHub URL or owner/repo shorthand:
    ///   "https://github.com/owner/repo"
    ///   "owner/repo"
    ///
    /// The repo must already be cloned — call `clone_public_url` first.
    /// This method does only local disk I/O.
    fn extract(&self, source: &str) -> Result<Vec<NormalizedChunk>, ConnectorError> {
        let r    = RepoRef::parse(source).map_err(map_clone_error)?;
        let root = repo_path(&r);

        if !is_cloned(&r) {
            return Err(ConnectorError::NotConfigured(format!(
                "repo '{}' is not cloned — call clone_public_url first",
                r.key()
            )));
        }

        // Build the directory tree to get all files with metadata.
        let tree = build_tree_from_path(&root, &r.repo)
            .map_err(|e| ConnectorError::Io(
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            ))?;

        let all_files = flatten_files(&tree);

        let mut all_chunks: Vec<NormalizedChunk> = Vec::new();
        let mut chunk_index = 0usize;
        let mut skipped     = 0usize;
        let mut parsed_count = 0usize;

        for rel_path in &all_files {
            let abs_path = root.join(rel_path);

            // Skip files that are too large.
            if let Ok(meta) = std::fs::metadata(&abs_path) {
                if meta.len() > self.max_file_bytes {
                    skipped += 1;
                    continue;
                }
            }

            // Detect language from extension.
            let ext = abs_path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let language = Language::from_extension(&ext);

            // Skip non-source files (config, markdown, etc).
            if !language.is_source() {
                continue;
            }

            // Parse the file — skip on error (log but don't fail the whole batch).
            /*let parsed = match parse_file(&abs_path, &root, &language) {
                Ok(p)  => p,
                Err(e) => {
                    eprintln!("[codebase] skipping {rel_path}: {e}");
                    skipped += 1;
                    continue;
                }
            };*/
            let mut parsed = match parse_file(&abs_path, &root, &language) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("[codebase] skipping {rel_path}: {e}");
                    skipped += 1;
                    continue;
                }
            };
            if let Ok(source) = std::fs::read_to_string(&abs_path) {
                enrich_with_calls(&mut parsed, &source, &[]);
            }

            // Normalize to chunks.
            let file_chunks = normalize_file(&parsed, &r.owner, &r.repo, chunk_index);
            chunk_index     += file_chunks.len();
            parsed_count    += 1;
            all_chunks.extend(file_chunks);
        }

        println!(
            "[codebase] {}/{} files parsed → {} chunks ({} skipped)",
            parsed_count,
            all_files.len(),
            all_chunks.len(),
            skipped,
        );

        Ok(all_chunks)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_registry::codebase::clone::TestReposRootGuard;
    use std::fs;
    use uuid::Uuid;

    /// Writes the standard fixture files under `tmp/` (unique per caller).
    fn populate_tmp_repo(tmp: &std::path::Path) {
        fs::create_dir_all(tmp.join("src")).unwrap();

        fs::write(
            tmp.join("src/lib.rs"),
            r#"
use std::collections::HashMap;
use crate::graph::Node;

pub struct Graph {
    nodes: HashMap<String, Node>,
}

impl Graph {
    pub fn new() -> Self { Self { nodes: HashMap::new() } }
    pub fn node_count(&self) -> usize { self.nodes.len() }
}
"#,
        )
        .unwrap();

        fs::write(
            tmp.join("src/main.rs"),
            r#"
use crate::graph::Graph;

fn main() {
    let g = Graph::new();
    println!("{}", g.node_count());
}
"#,
        )
        .unwrap();

        // Non-source file — should be skipped.
        fs::write(tmp.join("README.md"), "# test repo").unwrap();
        // Large file — should be skipped.
        fs::write(tmp.join("src/big.rs"), "x".repeat(600 * 1024)).unwrap();
    }

    /// Unique temp source tree + unique `{repos_root}/{owner}/{repo}/` so tests
    /// can run in parallel. `repos_root` is injected per-thread (not `~/.fluvio/repos`)
    /// so sandboxes and CI can write without touching the real home directory.
    fn isolated_clone_fixture() -> (std::path::PathBuf, RepoRef, TestReposRootGuard) {
        let id = Uuid::new_v4();
        let owner = format!("ctest-o-{id}");
        let repo_name = format!("ctest-r-{id}");
        let r = RepoRef::parse(&format!("{owner}/{repo_name}")).unwrap();

        let tmp = std::env::temp_dir().join(format!("fluvio_connector_test_{id}"));
        let _ = fs::remove_dir_all(&tmp);
        populate_tmp_repo(&tmp);

        let repos_root = std::env::temp_dir().join(format!("fluvio_connector_repos_{id}"));
        let _ = fs::remove_dir_all(&repos_root);
        fs::create_dir_all(&repos_root).unwrap();
        let guard = TestReposRootGuard::new(repos_root);

        let dest = repo_path(&r);
        let _ = fs::remove_dir_all(&dest);
        fs::create_dir_all(dest.join("src")).unwrap();

        for entry in fs::read_dir(tmp.join("src")).unwrap() {
            let entry = entry.unwrap();
            if entry.metadata().unwrap().is_file() {
                fs::copy(entry.path(), dest.join("src").join(entry.file_name())).unwrap();
            }
        }

        let readme = tmp.join("README.md");
        if readme.exists() {
            fs::copy(&readme, dest.join("README.md")).unwrap();
        }

        (tmp, r, guard)
    }

    fn url_for(r: &RepoRef) -> String {
        format!("{}/{}", r.owner, r.repo)
    }

    #[test]
    fn test_connector_meta() {
        let c = CodebaseConnector::new();
        assert_eq!(c.domain(), Domain::Codebase);
        assert_eq!(c.name(), "codebase");
    }

    #[test]
    fn test_extract_not_cloned_error() {
        let c = CodebaseConnector::new();
        let err = c.extract("https://github.com/fake/repo-that-does-not-exist-xyz")
            .unwrap_err();
        assert!(matches!(err, ConnectorError::NotConfigured(_)));
    }

    #[test]
    fn test_extract_produces_chunks() {
        let (tmp, r, _guard) = isolated_clone_fixture();

        let c      = CodebaseConnector::new();
        let url    = url_for(&r);
        let chunks = c.extract(&url).unwrap();

        // Should have chunks from lib.rs and main.rs (big.rs skipped, README.md skipped).
        assert!(!chunks.is_empty());

        // Every chunk should be in the codebase domain.
        assert!(chunks.iter().all(|c| c.domain == Domain::Codebase));

        // Every chunk should have source=codebase metadata.
        assert!(chunks
            .iter()
            .all(|c| c.metadata.get("source").map(|s| s == "codebase").unwrap_or(false)));

        // Should have both file-level and symbol-level chunks.
        let file_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.metadata.get("kind").map(|k| k == "file").unwrap_or(false))
            .collect();
        assert!(file_chunks.len() >= 2, "expected at least 2 file chunks");

        // Cleanup.
        let _ = fs::remove_dir_all(repo_path(&r));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_extract_skips_large_files() {
        let (tmp, r, _guard) = isolated_clone_fixture();

        // Copy big.rs into the fake clone too.
        fs::copy(
            tmp.join("src/big.rs"),
            repo_path(&r).join("src/big.rs"),
        )
        .unwrap();

        let c      = CodebaseConnector::new();
        let url    = url_for(&r);
        let chunks = c.extract(&url).unwrap();

        // No chunk should come from big.rs.
        assert!(!chunks
            .iter()
            .any(|c| c.metadata.get("path").map(|p| p.contains("big.rs")).unwrap_or(false)));

        let _ = fs::remove_dir_all(repo_path(&r));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_extract_file_single() {
        let (tmp, r, _guard) = isolated_clone_fixture();

        let url    = url_for(&r);
        let chunks = CodebaseConnector::extract_file(&url, "src/lib.rs").unwrap();

        // Should have file chunk + symbol chunks for Graph struct + Graph impl + new + node_count.
        assert!(!chunks.is_empty());
        assert!(chunks
            .iter()
            .any(|c| c.metadata.get("path").map(|p| p == "src/lib.rs").unwrap_or(false)));

        let _ = fs::remove_dir_all(repo_path(&r));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_extract_file_non_source_errors() {
        let (tmp, r, _guard) = isolated_clone_fixture();

        let url = url_for(&r);
        let err = CodebaseConnector::extract_file(&url, "README.md").unwrap_err();
        assert!(matches!(err, ConnectorError::Parse(_)));

        let _ = fs::remove_dir_all(repo_path(&r));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_chunk_indices_sequential() {
        let (tmp, r, _guard) = isolated_clone_fixture();

        let c      = CodebaseConnector::new();
        let url    = url_for(&r);
        let chunks = c.extract(&url).unwrap();

        // Chunk indices should be monotonically increasing.
        let indices: Vec<usize> = chunks.iter().map(|c| c.chunk_index).collect();
        for w in indices.windows(2) {
            assert!(w[1] > w[0], "chunk indices should be strictly increasing");
        }

        let _ = fs::remove_dir_all(repo_path(&r));
        let _ = fs::remove_dir_all(&tmp);
    }
}