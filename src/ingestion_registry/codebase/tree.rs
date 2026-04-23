//! tree.rs
//! 
//! Reads a cloned repo on disk and produces a heirarichal `ModuleTree` 
//! the frontend uses to render the solar system 
//! 
//! v1: directory-structure based - folders = planets, files = moons.
//! v2 (future): language aware - Cargo.toml / pyproject.toml / package.json 
//!         define crate/package boundaries instead of raw directories. 
//! 
//! Output shape (frontend receives the following): 
//! {
//!    "name": "kg-engine",
//!   "path": "",
//!   "kind": "repo",
//!   "size_bytes": 284000,
//!   "file_count": 42,
//!   "language": "rust",         ← dominant language
//!   "children": [
//!     {
//!       "name": "src",
//!       "path": "src",
//!       "kind": "module",
//!       "size_bytes": 220000,
//!       "file_count": 38,
//!       "language": "rust",
//!       "children": [ ... ]
//!     },
//!     {
//!       "name": "Cargo.toml",
//!       "path": "Cargo.toml",
//!       "kind": "file",
//!       "size_bytes": 1200,
//!       "file_count": 1,
//!       "language": "toml",
//!       "children": []
//!     }
//!   ]
//! } 
//! 

use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::clone::{CloneError, RepoRef, repo_path};

// --- Error -----------------------------------------------------
#[derive(Debug, Error)]
pub enum TreeError {
    #[error("repo not cloned: {0}")]
    NotCloned(String),
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] CloneError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ---- Language Detection ----------------------------------------

// Detected language for a file or dominant language for a directory. 
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    C,
    Cpp,
    Toml,
    Json,
    Yaml,
    Markdown,
    Html,
    Css,
    Shell,
    Other,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs"                            => Language::Rust,
            "py" | "pyi"                    => Language::Python,
            "ts" | "tsx"                    => Language::TypeScript,
            "js" | "jsx" | "mjs"            => Language::JavaScript,
            "go"                            => Language::Go,
            "java"                          => Language::Java,
            "c" | "h"                       => Language::C,
            "cpp" | "cc" | "cxx" | "hpp"    => Language:: Cpp,
            "toml"                          => Language::Toml,
            "json"                          => Language::Json,
            "yaml" | "yml"                  => Language::Yaml,
            "md" | "mdx"                    => Language::Markdown,
            "html" | "htm"                  => Language::Html,
            "css" | "scss" | "sass"         => Language::Css,
            "sh" | "bash" | "zsh"           => Language::Shell,
            _                               => Language::Other,
        }
    }

    // Returns true for languages we can parse for imports in future
    pub fn is_source(&self) -> bool {
        matches!(
            self,
            Language::Rust
                | Language::Python
                | Language::TypeScript
                | Language::JavaScript
                | Language::Go
                | Language::Java
                | Language::C
                | Language::Cpp
        )
    }
}

// Compute the dominant language across a frequent map.
fn dominant_language(counts: &HashMap<Language, usize>) -> Language {
    let source_dominant = counts.iter().filter(|(lang, _)| lang.is_source())
                                .max_by_key(|(_, count)| *count)
                                .map(|(lang, _)| lang.clone());

    source_dominant.unwrap_or_else(|| {
        counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.clone())
            .unwrap_or(Language::Other)
    })
}

// ---- Tree Node --------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// The root repo node
    Repo, 
    // This is a planet in the solar system.
    Module,
    // A single file - becomes a moon orbiting its parent module.
    File,
}

/// One node in the module tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub kind: NodeKind,

    pub size_bytes: u64,
    pub file_count: usize,

    pub language: Language,
    // Depth from repo root (0 = top-level children of repo)
    pub depth: usize,

    /// child nodes - empty for files
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn is_parseable(&self) -> bool {
        self.kind == NodeKind::File && self.language.is_source()
    }
}

// ── Dirs/files to always skip ─────────────────────────────────────────────────
fn should_skip(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".github"
            | "target"
            | "node_modules"
            | ".next"
            | "dist"
            | "build"
            | "__pycache__"
            | ".mypy_cache"
            | ".pytest_cache"
            | ".venv"
            | "venv"
            | ".env"
            | "vendor"
            | ".idea"
            | ".vscode"
    )
}

///! Public API -----------------------------------------------------
/// 
/// 
/// Build a `TreeNode` (repo root) from a cloned repo URL
/// `url` is parsed the same way as `clone_or_pull`
pub fn build_tree(url: &str) -> Result<TreeNode, TreeError> {
    let r = RepoRef::parse(url)?;

    let root = repo_path(&r);

    if !root.is_dir() {
        return Err(TreeError::NotCloned(format!(
            "Expected clone at {}", 
            root.display()
        )));
    }

    let mut node = walk_dir(&root, &root, 0)?;

    // Override the root node's name/kind to represent repo
    node.name = r.repo.clone();
    node.path = String::new();
    node.kind = NodeKind::Repo;

    Ok(node)
}

pub fn build_tree_from_path(root: &Path, repo_name: &str) -> Result<TreeNode, TreeError> {
    if !root.is_dir() {
        return Err(TreeError::NotCloned(format!(
            "Expected clone at {}", 
            root.display()
        )));
    }

    let mut node = walk_dir(root, root, 0)?;
    node.name = repo_name.to_string();
    node.path = String::new();
    node.kind = NodeKind::Repo;

    Ok(node)
}

///! Internal Walk -----------------------------------------------
fn walk_dir(base: &Path, dir: &Path, depth: usize) -> Result<TreeNode, TreeError> {
    let name = dir
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "root".to_string());
    


    let rel_path = dir
                            .strip_prefix(base)
                            .map(|p| p.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default();
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
                                        .filter_map(|e| e.ok())
                                        .collect();
    
    // Sort: directories first, then files , both alphabatically.
    entries.sort_by(|a, b| {
        let a_is_dir = a.metadata().map(|m| m.is_dir()).unwrap_or(false);
        let b_is_dir = b.metadata().map(|m| m.is_dir()).unwrap_or(false);

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    let mut children: Vec<TreeNode> = Vec::new();
    let mut lang_counts: HashMap<Language, usize> = HashMap::new();
    let mut total_size: u64 = 0;
    let mut total_files: usize = 0;

    for entry in entries {
        let entry_name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&entry_name) {
            continue;
        }

        let meta = entry.metadata()?;
        let path = entry.path();

        if meta.is_dir() {
            let child = walk_dir(base, &path, depth + 1)?;
            total_size += child.size_bytes;
            total_files += child.file_count;

            //Merge language counts from child
            *lang_counts.entry(child.language.clone()).or_insert(0) += child.file_count;

            children.push(child);
        }else if meta.is_file() {
            let ext = path.extension().map(|e| e.to_string_lossy().to_string())
                                                .unwrap_or_default();
            let lang = Language::from_extension(&ext);
            let size = meta.len();

            total_size += size;
            total_files += 1;
            *lang_counts.entry(lang.clone()).or_insert(0) += 1;

            let rel_file = path.strip_prefix(base)
                                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                                        .unwrap_or_default();
            
            children.push(TreeNode {
                name:                   entry_name,
                path:                   rel_file,
                kind:                   NodeKind::File,
                size_bytes:             size,
                file_count:             1,
                language:               lang,
                depth:                  depth + 1,
                children:               vec![],
            });
        }
    }

    let language = dominant_language(&lang_counts);


    Ok(TreeNode {
        name,
        path: rel_path,
        kind: NodeKind::Module,
        size_bytes: total_size,
        file_count: total_files,
        language,
        depth,
        children,
    })
}

///! Flat file list (for existing server endpoints)
///! Returns a flat sorted list of all file paths in the tree.
/// Useful for existing `/sync/codebase/files` endpoint.
pub fn flatten_files(tree: &TreeNode) -> Vec<String> {
    let mut out = Vec::new();
    collect_files(tree, &mut out);
    out.sort();
    out
}

fn collect_files(node: &TreeNode, out: &mut Vec<String>) {
    if node.kind == NodeKind::File {
        out.push(node.path.clone());
        return;
    }else {
        for child in &node.children {
            collect_files(child, out);

        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
 
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
 
    /// Create a temp directory tree for testing.
    fn make_test_repo(tmp: &Path) {
        // src/
        //   main.rs
        //   lib.rs
        //   ingestion/
        //     mod.rs
        //     email.rs
        // Cargo.toml
        // README.md
        // .git/               ← should be skipped
        //   config
        // target/             ← should be skipped
        //   debug/
 
        fs::create_dir_all(tmp.join("src/ingestion")).unwrap();
        fs::create_dir_all(tmp.join(".git")).unwrap();
        fs::create_dir_all(tmp.join("target/debug")).unwrap();
 
        fs::write(tmp.join("src/main.rs"),           "fn main() {}").unwrap();
        fs::write(tmp.join("src/lib.rs"),            "pub mod ingestion;").unwrap();
        fs::write(tmp.join("src/ingestion/mod.rs"),  "pub mod email;").unwrap();
        fs::write(tmp.join("src/ingestion/email.rs"), "// email").unwrap();
        fs::write(tmp.join("Cargo.toml"),             "[package]").unwrap();
        fs::write(tmp.join("README.md"),              "# test").unwrap();
        fs::write(tmp.join(".git/config"),            "[core]").unwrap();
        fs::write(tmp.join("target/debug/bin"),       "binary").unwrap();
    }
 
    #[test]
    fn test_build_tree_structure() {
        let tmp = std::env::temp_dir().join("fluvio_tree_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        make_test_repo(&tmp);
 
        let tree = build_tree_from_path(&tmp, "test-repo").unwrap();
 
        assert_eq!(tree.kind,  NodeKind::Repo);
        assert_eq!(tree.name,  "test-repo");
        assert_eq!(tree.path,  "");
        assert_eq!(tree.depth, 0);
 
        // .git and target should be skipped.
        assert!(!tree.children.iter().any(|c| c.name == ".git"));
        assert!(!tree.children.iter().any(|c| c.name == "target"));
 
        // Should have src/, Cargo.toml, README.md.
        let names: Vec<&str> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"src"), "expected src in {:?}", names);
        assert!(names.contains(&"Cargo.toml"));
        assert!(names.contains(&"README.md"));
 
        let _ = fs::remove_dir_all(&tmp);
    }
 
    #[test]
    fn test_directories_before_files() {
        let tmp = std::env::temp_dir().join("fluvio_tree_sort_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        make_test_repo(&tmp);
 
        let tree = build_tree_from_path(&tmp, "test-repo").unwrap();
 
        // src/ (dir) should come before Cargo.toml (file).
        let first = &tree.children[0];
        assert_eq!(first.kind, NodeKind::Module, "first child should be a directory");
 
        let _ = fs::remove_dir_all(&tmp);
    }
 
    #[test]
    fn test_file_counts_and_sizes() {
        let tmp = std::env::temp_dir().join("fluvio_tree_counts_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        make_test_repo(&tmp);
 
        let tree = build_tree_from_path(&tmp, "test-repo").unwrap();
 
        // 4 Rust files + Cargo.toml + README.md = 6 files (excluding .git and target).
        assert_eq!(tree.file_count, 6, "expected 6 files, got {}", tree.file_count);
        assert!(tree.size_bytes > 0);
 
        let _ = fs::remove_dir_all(&tmp);
    }
 
    #[test]
    fn test_dominant_language_rust() {
        let tmp = std::env::temp_dir().join("fluvio_tree_lang_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        make_test_repo(&tmp);
 
        let tree = build_tree_from_path(&tmp, "test-repo").unwrap();
 
        // 4 .rs files vs 1 .toml vs 1 .md → Rust should dominate.
        assert_eq!(tree.language, Language::Rust);
 
        let _ = fs::remove_dir_all(&tmp);
    }
 
    #[test]
    fn test_nested_depth() {
        let tmp = std::env::temp_dir().join("fluvio_tree_depth_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        make_test_repo(&tmp);
 
        let tree = build_tree_from_path(&tmp, "test-repo").unwrap();
 
        // src/ is depth 1, src/ingestion/ is depth 2, email.rs is depth 3.
        let src = tree.children.iter().find(|c| c.name == "src").unwrap();
        assert_eq!(src.depth, 1);
 
        let ingestion = src.children.iter().find(|c| c.name == "ingestion").unwrap();
        assert_eq!(ingestion.depth, 2);
 
        let email = ingestion.children.iter().find(|c| c.name == "email.rs").unwrap();
        assert_eq!(email.depth, 3);
        assert_eq!(email.kind, NodeKind::File);
        assert_eq!(email.language, Language::Rust);
 
        let _ = fs::remove_dir_all(&tmp);
    }
 
    #[test]
    fn test_flatten_files() {
        let tmp = std::env::temp_dir().join("fluvio_tree_flatten_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        make_test_repo(&tmp);
 
        let tree  = build_tree_from_path(&tmp, "test-repo").unwrap();
        let files = flatten_files(&tree);
 
        // Should contain all 6 files sorted.
        assert_eq!(files.len(), 6);
        assert!(files.iter().any(|f| f.ends_with("main.rs")));
        assert!(files.iter().any(|f| f.ends_with("email.rs")));
        assert!(files.iter().any(|f| f == "Cargo.toml"));
 
        let _ = fs::remove_dir_all(&tmp);
    }
 
    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"),   Language::Rust);
        assert_eq!(Language::from_extension("py"),   Language::Python);
        assert_eq!(Language::from_extension("ts"),   Language::TypeScript);
        assert_eq!(Language::from_extension("js"),   Language::JavaScript);
        assert_eq!(Language::from_extension("go"),   Language::Go);
        assert_eq!(Language::from_extension("toml"), Language::Toml);
        assert_eq!(Language::from_extension("md"),   Language::Markdown);
        assert_eq!(Language::from_extension("xyz"),  Language::Other);
    }
 
    #[test]
    fn test_should_skip() {
        assert!(should_skip(".git"));
        assert!(should_skip("target"));
        assert!(should_skip("node_modules"));
        assert!(should_skip("__pycache__"));
        assert!(!should_skip("src"));
        assert!(!should_skip("main.rs"));
    }
 
    #[test]
    fn test_not_cloned_error() {
        let result = build_tree_from_path(
            Path::new("/tmp/fluvio_definitely_does_not_exist_xyz"),
            "fake",
        );
        assert!(matches!(result, Err(TreeError::NotCloned(_))));
    }
 
    #[test]
    fn test_file_node_not_parseable_for_toml() {
        let node = TreeNode {
            name:       "Cargo.toml".to_string(),
            path:       "Cargo.toml".to_string(),
            kind:       NodeKind::File,
            size_bytes: 100,
            file_count: 1,
            language:   Language::Toml,
            depth:      0,
            children:   vec![],
        };
        assert!(!node.is_parseable());
    }
 
    #[test]
    fn test_rust_file_is_parseable() {
        let node = TreeNode {
            name:       "main.rs".to_string(),
            path:       "src/main.rs".to_string(),
            kind:       NodeKind::File,
            size_bytes: 500,
            file_count: 1,
            language:   Language::Rust,
            depth:      1,
            children:   vec![],
        };
        assert!(node.is_parseable());
    }
}
 