//! parser.rs
//!
//! Extracts imports and top-level symbols from Rust, Python, and TypeScript
//! source files using regex-based parsing.
//!
//! No AST / tree-sitter in v1 — regex is sufficient for top-level definitions
//! and import statements across all three languages.
//!
//! Output is language-agnostic: every file produces a `ParsedFile` with
//! `Vec<Import>` and `Vec<Symbol>` regardless of source language.

use std::path::Path;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::tree::Language;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("IO error reading {path}: {source}")]
    Io {
        path:   String,
        source: std::io::Error,
    },
    #[error("unsupported language: {0:?}")]
    UnsupportedLanguage(Language),
    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),
}

// ── Output types ──────────────────────────────────────────────────────────────

/// A resolved import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// The raw import string as written in the source.
    pub raw: String,
    /// Resolved relative file path within the repo, if we can determine it.
    /// e.g. "use crate::graph::structs" → Some("src/graph/structs.rs")
    pub resolved: Option<String>,
    /// Line number (1-based).
    pub line: usize,
}

/// A top-level symbol extracted from the file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name:      String,
    pub kind:      SymbolKind,
    /// Full signature line as written in the source.
    pub signature: String,
    /// Line number (1-based).
    pub line:      usize,
    pub is_public: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Impl,
    Trait,
    Class,
    Method,
    TypeAlias,
    Interface,
    Constant,
}

/// Full parse result for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub path:     String,
    pub language: Language,
    pub imports:  Vec<Import>,
    pub symbols:  Vec<Symbol>,
}

impl ParsedFile {
    /// All public symbols — what this file exposes to the rest of the codebase.
    pub fn public_api(&self) -> Vec<&Symbol> {
        self.symbols.iter().filter(|s| s.is_public).collect()
    }

    /// All import paths that look like they reference internal repo files
    /// (i.e. have a resolved path).
    pub fn internal_imports(&self) -> Vec<&Import> {
        self.imports.iter().filter(|i| i.resolved.is_some()).collect()
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse a file at `abs_path` within a repo rooted at `repo_root`.
/// `language` is determined by the caller (from `tree::Language::from_extension`).
pub fn parse_file(
    abs_path: &Path,
    repo_root: &Path,
    language: &Language,
) -> Result<ParsedFile, ParseError> {
    let source = std::fs::read_to_string(abs_path).map_err(|e| ParseError::Io {
        path:   abs_path.display().to_string(),
        source: e,
    })?;

    let rel_path = abs_path
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| abs_path.to_string_lossy().to_string());

    let (imports, symbols) = match language {
        Language::Rust       => parse_rust(&source, repo_root),
        Language::Python     => parse_python(&source),
        Language::TypeScript | Language::JavaScript => parse_typescript(&source),
        other => return Err(ParseError::UnsupportedLanguage(other.clone())),
    }?;

    Ok(ParsedFile {
        path: rel_path,
        language: language.clone(),
        imports,
        symbols,
    })
}

// ── Rust parser ───────────────────────────────────────────────────────────────

fn parse_rust(
    source: &str,
    repo_root: &Path,
) -> Result<(Vec<Import>, Vec<Symbol>), ParseError> {
    let mut imports = Vec::new();
    let mut symbols = Vec::new();

    // use crate::graph::structs::{Node, Edge};
    // use super::clone::RepoRef;
    // use std::collections::HashMap;   ← external, resolved = None
    let use_re = Regex::new(r"^(?:pub\s+)?use\s+(.+?);")
        .map_err(ParseError::Regex)?;

    // pub fn foo(...) / fn foo(...)
    let fn_re = Regex::new(
        r"^(?P<vis>pub(?:\([\w:]+\))?\s+)?(?:async\s+)?fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)",
    )
    .map_err(ParseError::Regex)?;

    // pub struct Foo / struct Foo
    let struct_re =
        Regex::new(r"^(?P<vis>pub(?:\([\w:]+\))?\s+)?struct\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
            .map_err(ParseError::Regex)?;

    // pub enum Foo / enum Foo
    let enum_re =
        Regex::new(r"^(?P<vis>pub(?:\([\w:]+\))?\s+)?enum\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
            .map_err(ParseError::Regex)?;

    // impl Foo / impl<T> Foo / impl Bar for Foo
    let impl_re = Regex::new(r"^impl(?:<[^>]*>)?\s+(?:[A-Za-z_][A-Za-z0-9_<>, ]*\s+for\s+)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
        .map_err(ParseError::Regex)?;

    // pub trait Foo / trait Foo
    let trait_re =
        Regex::new(r"^(?P<vis>pub(?:\([\w:]+\))?\s+)?trait\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
            .map_err(ParseError::Regex)?;

    // pub type Foo = ...
    let type_re =
        Regex::new(r"^(?P<vis>pub(?:\([\w:]+\))?\s+)?type\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
            .map_err(ParseError::Regex)?;

    // pub const FOO: ...  / pub static FOO: ...
    let const_re = Regex::new(
        r"^(?P<vis>pub(?:\([\w:]+\))?\s+)?(?:const|static)\s+(?P<name>[A-Z_][A-Z0-9_]*)",
    )
    .map_err(ParseError::Regex)?;

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();

        // Skip comments and empty lines.
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }

        // Imports.
        if let Some(cap) = use_re.captures(trimmed) {
            let path_str = cap[1].trim().to_string();
            let resolved = resolve_rust_import(&path_str, repo_root);
            imports.push(Import {
                raw:      format!("use {path_str}"),
                resolved,
                line:     line_no,
            });
            continue;
        }

        // Functions.
        if let Some(cap) = fn_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Function,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        // Structs.
        if let Some(cap) = struct_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Struct,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        // Enums.
        if let Some(cap) = enum_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Enum,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        // Impls.
        if let Some(cap) = impl_re.captures(trimmed) {
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Impl,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public: false, // impls don't have visibility
            });
            continue;
        }

        // Traits.
        if let Some(cap) = trait_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Trait,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        // Type aliases.
        if let Some(cap) = type_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::TypeAlias,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        // Constants.
        if let Some(cap) = const_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Constant,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
        }
    }

    Ok((imports, symbols))
}

/// Try to resolve a Rust `use` path to a relative file path in the repo.
/// `use crate::graph::structs` → `src/graph/structs.rs` (if it exists)
/// `use std::collections::HashMap` → None (external)
fn resolve_rust_import(path: &str, repo_root: &Path) -> Option<String> {
    // Strip leading `crate::`, `super::`, `self::`.
    let stripped = path
        .split('{').next()           // handle `use foo::{A, B}` — take the prefix
        .unwrap_or(path)
        .trim_end_matches("::")
        .trim();

    let without_prefix = stripped
        .strip_prefix("crate::")
        .or_else(|| stripped.strip_prefix("super::"))
        .or_else(|| stripped.strip_prefix("self::"))?;

    // Convert `graph::structs` → `src/graph/structs`
    let rel = without_prefix.replace("::", "/");

    // Try src/{rel}.rs first, then {rel}.rs, then src/{rel}/mod.rs.
    let candidates = [
        format!("src/{rel}.rs"),
        format!("{rel}.rs"),
        format!("src/{rel}/mod.rs"),
        format!("{rel}/mod.rs"),
    ];

    for candidate in &candidates {
        if repo_root.join(candidate).exists() {
            return Some(candidate.clone());
        }
    }

    None
}

// ── Python parser ─────────────────────────────────────────────────────────────

fn parse_python(source: &str) -> Result<(Vec<Import>, Vec<Symbol>), ParseError> {
    let mut imports = Vec::new();
    let mut symbols = Vec::new();

    // import os / import os.path
    let import_re = Regex::new(r"^import\s+(.+)").map_err(ParseError::Regex)?;
    // from os.path import join / from . import something
    let from_re = Regex::new(r"^from\s+(\S+)\s+import\s+(.+)").map_err(ParseError::Regex)?;

    // def foo(...): / async def foo(...):
    let fn_re = Regex::new(r"^(?P<vis>)?(?:async\s+)?def\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
        .map_err(ParseError::Regex)?;

    // class Foo: / class Foo(Bar):
    let class_re =
        Regex::new(r"^class\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)").map_err(ParseError::Regex)?;

    // FOO = ... / FOO: int = ...  (module-level constants — UPPER_CASE only)
    let const_re =
        Regex::new(r"^(?P<name>[A-Z_][A-Z0-9_]+)\s*(?::\s*\S+\s*)?=").map_err(ParseError::Regex)?;

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // `from x import y` — check before plain `import`.
        if let Some(cap) = from_re.captures(trimmed) {
            let module = cap[1].trim().to_string();
            let names  = cap[2].trim().to_string();
            imports.push(Import {
                raw:      format!("from {module} import {names}"),
                resolved: resolve_python_import(&module),
                line:     line_no,
            });
            continue;
        }

        if let Some(cap) = import_re.captures(trimmed) {
            let module = cap[1].trim().to_string();
            imports.push(Import {
                raw:      format!("import {module}"),
                resolved: resolve_python_import(&module),
                line:     line_no,
            });
            continue;
        }

        // Top-level only — skip indented lines (they're methods/inner functions).
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(cap) = class_re.captures(trimmed) {
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Class,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public: !cap["name"].starts_with('_'),
            });
            continue;
        }

        if let Some(cap) = fn_re.captures(trimmed) {
            let name = cap["name"].to_string();
            let is_public = !name.starts_with('_');
            symbols.push(Symbol {
                name,
                kind:      SymbolKind::Function,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        if let Some(cap) = const_re.captures(trimmed) {
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Constant,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public: true,
            });
        }
    }

    Ok((imports, symbols))
}

/// Resolve a Python import to a relative file path.
/// `from .email import connector` → `email/connector.py`
/// `from anthropic import Anthropic` → None (external)
fn resolve_python_import(module: &str) -> Option<String> {
    // Relative imports start with `.`
    if module.starts_with('.') {
        let clean = module.trim_start_matches('.');
        let rel   = clean.replace('.', "/");
        // Could be a file or a package.
        return Some(if rel.is_empty() {
            "__init__.py".to_string()
        } else {
            format!("{rel}.py")
        });
    }
    // We can't resolve absolute imports without knowing site-packages.
    None
}

// ── TypeScript / JavaScript parser ────────────────────────────────────────────

fn parse_typescript(source: &str) -> Result<(Vec<Import>, Vec<Symbol>), ParseError> {
    let mut imports = Vec::new();
    let mut symbols = Vec::new();

    // import { foo } from './bar'
    // import foo from '../baz'
    // import * as foo from '../../qux'
    // import './side-effect'
    let import_re =
        Regex::new(r#"^import\s+(?:[^'"]*\s+from\s+)?['"]([^'"]+)['"]"#)
            .map_err(ParseError::Regex)?;

    // export { foo } from './bar'  (re-export)
    let reexport_re =
        Regex::new(r#"^export\s+\{[^}]*\}\s+from\s+['"]([^'"]+)['"]"#)
            .map_err(ParseError::Regex)?;

    // function foo(  /  export function foo(  /  export default function foo(
    let fn_re = Regex::new(
        r"^(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)",
    )
    .map_err(ParseError::Regex)?;

    // export const foo = / const foo =  (arrow functions and values)
    let const_fn_re = Regex::new(
        r"^(?P<vis>export\s+)?(?:const|let|var)\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)\s*(?::\s*[^=]+)?\s*=\s*(?:async\s+)?(?:\([^)]*\)|[A-Za-z_$][A-Za-z0-9_$]*)\s*(?::\s*[^=>{]+)?\s*=>",
    )
    .map_err(ParseError::Regex)?;

    // class Foo / export class Foo / export default class Foo
    let class_re = Regex::new(
        r"^(?:export\s+)?(?:default\s+)?(?:abstract\s+)?class\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)",
    )
    .map_err(ParseError::Regex)?;

    // export interface Foo / interface Foo
    let interface_re =
        Regex::new(r"^(?:export\s+)?interface\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)")
            .map_err(ParseError::Regex)?;

    // export type Foo = / type Foo =
    let type_re =
        Regex::new(r"^(?P<vis>export\s+)?type\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)\s*[=<]")
            .map_err(ParseError::Regex)?;

    // export const FOO = (uppercase constants)
    let const_re = Regex::new(
        r"^(?P<vis>export\s+)?(?:const|let)\s+(?P<name>[A-Z_$][A-Z0-9_$]*)\s*(?::\s*[^=]+)?\s*=",
    )
    .map_err(ParseError::Regex)?;

    for (line_idx, line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("*")
            || trimmed.starts_with("/*")
        {
            continue;
        }

        // Imports.
        if let Some(cap) = import_re.captures(trimmed) {
            let path = cap[1].to_string();
            imports.push(Import {
                raw:      trimmed.chars().take(120).collect(),
                resolved: resolve_ts_import(&path),
                line:     line_no,
            });
            continue;
        }

        if let Some(cap) = reexport_re.captures(trimmed) {
            let path = cap[1].to_string();
            imports.push(Import {
                raw:      trimmed.chars().take(120).collect(),
                resolved: resolve_ts_import(&path),
                line:     line_no,
            });
            continue;
        }

        // Symbols — skip indented lines (they're class methods etc).
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }

        if let Some(cap) = class_re.captures(trimmed) {
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Class,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        if let Some(cap) = interface_re.captures(trimmed) {
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Interface,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        if let Some(cap) = type_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::TypeAlias,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        if let Some(cap) = fn_re.captures(trimmed) {
            let is_public = trimmed.starts_with("export");
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Function,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        if let Some(cap) = const_fn_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Function,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
            continue;
        }

        if let Some(cap) = const_re.captures(trimmed) {
            let is_public = cap.name("vis").is_some();
            symbols.push(Symbol {
                name:      cap["name"].to_string(),
                kind:      SymbolKind::Constant,
                signature: trimmed.chars().take(120).collect(),
                line:      line_no,
                is_public,
            });
        }
    }

    Ok((imports, symbols))
}

/// Resolve a TypeScript/JS import path to a relative file path.
/// `./graph/structs` → `graph/structs.ts`
/// `../lib/utils` → `lib/utils.ts`
/// `react` → None (external npm package)
fn resolve_ts_import(path: &str) -> Option<String> {
    // Only resolve relative imports (start with . or /)
    if !path.starts_with('.') && !path.starts_with('/') {
        return None;
    }

    // Strip leading `./`
    let clean = path
        .trim_start_matches('/')
        .trim_start_matches("./");

    // If already has extension, return as-is.
    if clean.ends_with(".ts")
        || clean.ends_with(".tsx")
        || clean.ends_with(".js")
        || clean.ends_with(".jsx")
    {
        return Some(clean.to_string());
    }

    // Try common extensions.
    Some(format!("{clean}.ts"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ── Rust ──────────────────────────────────────────────────────────────────

    #[test]
    fn test_rust_imports() {
        let source = r#"
use std::collections::HashMap;
use crate::graph::structs::{Node, Edge};
use super::clone::RepoRef;
pub use crate::tree::TreeNode;
"#;
        let (imports, _) = parse_rust(source, Path::new("/repo")).unwrap();
        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].raw, "use std::collections::HashMap");
        assert!(imports[0].resolved.is_none()); // external
        assert_eq!(imports[1].raw, "use crate::graph::structs::{Node, Edge}");
    }

    #[test]
    fn test_rust_functions() {
        let source = r#"
pub fn add_node(&mut self, node: Node) -> NodeId {
fn private_helper() -> bool {
pub async fn serve(api_key: String) -> anyhow::Result<()> {
"#;
        let (_, symbols) = parse_rust(source, Path::new("/repo")).unwrap();
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "add_node");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert!(symbols[0].is_public);
        assert_eq!(symbols[1].name, "private_helper");
        assert!(!symbols[1].is_public);
        assert_eq!(symbols[2].name, "serve");
        assert!(symbols[2].is_public);
    }

    #[test]
    fn test_rust_structs_enums_traits() {
        let source = r#"
pub struct Graph {
struct PrivateState {
pub enum Domain {
pub trait FluvioGraph {
trait Internal {
"#;
        let (_, symbols) = parse_rust(source, Path::new("/repo")).unwrap();
        assert_eq!(symbols.len(), 5);
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert!(symbols[0].is_public);
        assert_eq!(symbols[1].kind, SymbolKind::Struct);
        assert!(!symbols[1].is_public);
        assert_eq!(symbols[2].kind, SymbolKind::Enum);
        assert_eq!(symbols[3].kind, SymbolKind::Trait);
        assert_eq!(symbols[4].kind, SymbolKind::Trait);
    }

    #[test]
    fn test_rust_impl() {
        let source = r#"
impl Graph {
impl FluvioGraph for DomainGraph {
impl<T: Send> Registry<T> {
"#;
        let (_, symbols) = parse_rust(source, Path::new("/repo")).unwrap();
        assert_eq!(symbols.len(), 3);
        assert!(symbols.iter().all(|s| s.kind == SymbolKind::Impl));
    }

    #[test]
    fn test_rust_type_alias_and_const() {
        let source = r#"
pub type NodeId = Uuid;
pub const MAX_DEPTH: usize = 32;
pub static DEFAULT_MODEL: &str = "claude";
"#;
        let (_, symbols) = parse_rust(source, Path::new("/repo")).unwrap();
        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].kind, SymbolKind::TypeAlias);
        assert_eq!(symbols[1].kind, SymbolKind::Constant);
        assert_eq!(symbols[2].kind, SymbolKind::Constant);
    }

    #[test]
    fn test_rust_resolve_import() {
        // Create a temp repo structure to test resolution.
        let tmp = std::env::temp_dir().join("fluvio_parser_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src/graph")).unwrap();
        fs::write(tmp.join("src/graph/structs.rs"), "").unwrap();

        let resolved = resolve_rust_import("crate::graph::structs", &tmp);
        assert_eq!(resolved, Some("src/graph/structs.rs".to_string()));

        let external = resolve_rust_import("std::collections::HashMap", &tmp);
        assert!(external.is_none());

        let _ = fs::remove_dir_all(&tmp);
    }

    // ── Python ────────────────────────────────────────────────────────────────

    #[test]
    fn test_python_imports() {
        let source = r#"
import os
import os.path
from typing import List, Dict
from .email import connector
from ..utils import helper
from anthropic import Anthropic
"#;
        let (imports, _) = parse_python(source).unwrap();
        assert_eq!(imports.len(), 6);
        assert!(imports[0].resolved.is_none()); // os is external
        assert!(imports[3].resolved.is_some()); // .email is relative
        assert_eq!(imports[3].resolved, Some("email.py".to_string()));
        assert!(imports[5].resolved.is_none()); // anthropic is external
    }

    #[test]
    fn test_python_functions_and_classes() {
        let source = r#"
class MyClass:
    def method(self):
        pass

def public_function():
    pass

def _private():
    pass

async def async_fn():
    pass
"#;
        let (_, symbols) = parse_python(source).unwrap();
        // Only top-level: MyClass, public_function, _private, async_fn
        // method is indented → skipped
        assert!(symbols.iter().any(|s| s.name == "MyClass" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "public_function" && s.is_public));
        assert!(symbols.iter().any(|s| s.name == "_private" && !s.is_public));
        assert!(symbols.iter().any(|s| s.name == "async_fn"));
        assert!(!symbols.iter().any(|s| s.name == "method")); // indented, skipped
    }

    #[test]
    fn test_python_constants() {
        let source = r#"
MAX_RETRIES = 3
BASE_URL: str = "https://api.example.com"
not_a_constant = "lowercase"
"#;
        let (_, symbols) = parse_python(source).unwrap();
        assert!(symbols.iter().any(|s| s.name == "MAX_RETRIES" && s.kind == SymbolKind::Constant));
        assert!(symbols.iter().any(|s| s.name == "BASE_URL"));
        assert!(!symbols.iter().any(|s| s.name == "not_a_constant"));
    }

    // ── TypeScript ────────────────────────────────────────────────────────────

    #[test]
    fn test_ts_imports() {
        let source = r#"
import { useState } from 'react';
import type { FC } from 'react';
import GraphCanvas from './GraphCanvas';
import * as d3 from 'd3';
import '../styles/global.css';
"#;
        let (imports, _) = parse_typescript(source).unwrap();
        assert!(imports.iter().any(|i| i.raw.contains("react") && i.resolved.is_none()));
        assert!(imports.iter().any(|i| i.resolved == Some("GraphCanvas.ts".to_string())));
    }

    #[test]
    fn test_ts_functions_and_classes() {
        let source = r#"
export function fetchGraph(url: string): Promise<Graph> {
export async function loadData() {
function internalHelper() {
export class GraphCanvas extends Component {
export interface TreeNode {
export type Language = 'rust' | 'python';
export const MAX_NODES = 700;
"#;
        let (_, symbols) = parse_typescript(source).unwrap();
        assert!(symbols.iter().any(|s| s.name == "fetchGraph" && s.is_public));
        assert!(symbols.iter().any(|s| s.name == "internalHelper" && !s.is_public));
        assert!(symbols.iter().any(|s| s.name == "GraphCanvas" && s.kind == SymbolKind::Class));
        assert!(symbols.iter().any(|s| s.name == "TreeNode" && s.kind == SymbolKind::Interface));
        assert!(symbols.iter().any(|s| s.name == "Language" && s.kind == SymbolKind::TypeAlias));
        assert!(symbols.iter().any(|s| s.name == "MAX_NODES" && s.kind == SymbolKind::Constant));
    }

    #[test]
    fn test_ts_resolve_import() {
        assert_eq!(
            resolve_ts_import("./GraphCanvas"),
            Some("GraphCanvas.ts".to_string())
        );
        assert_eq!(
            resolve_ts_import("../lib/utils"),
            Some("../lib/utils.ts".to_string())
        );
        assert!(resolve_ts_import("react").is_none());
        assert!(resolve_ts_import("@/components/Button").is_none());
    }

    // ── ParsedFile helpers ────────────────────────────────────────────────────

    #[test]
    fn test_public_api_filter() {
        let file = ParsedFile {
            path:     "src/lib.rs".to_string(),
            language: Language::Rust,
            imports:  vec![],
            symbols:  vec![
                Symbol {
                    name: "public_fn".to_string(), kind: SymbolKind::Function,
                    signature: "pub fn public_fn()".to_string(), line: 1, is_public: true,
                },
                Symbol {
                    name: "private_fn".to_string(), kind: SymbolKind::Function,
                    signature: "fn private_fn()".to_string(), line: 2, is_public: false,
                },
            ],
        };
        let api = file.public_api();
        assert_eq!(api.len(), 1);
        assert_eq!(api[0].name, "public_fn");
    }

    #[test]
    fn test_internal_imports_filter() {
        let file = ParsedFile {
            path:     "src/server.rs".to_string(),
            language: Language::Rust,
            imports: vec![
                Import { raw: "use std::sync::Arc".to_string(), resolved: None, line: 1 },
                Import {
                    raw:      "use crate::graph::Graph".to_string(),
                    resolved: Some("src/graph/mod.rs".to_string()),
                    line:     2,
                },
            ],
            symbols: vec![],
        };
        let internal = file.internal_imports();
        assert_eq!(internal.len(), 1);
        assert_eq!(internal[0].resolved, Some("src/graph/mod.rs".to_string()));
    }

    // ── parse_file integration ────────────────────────────────────────────────

    #[test]
    fn test_parse_file_rust() {
        let tmp = std::env::temp_dir().join("fluvio_parse_file_test");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();

        let content = r#"
use std::collections::HashMap;
use crate::graph::Node;

pub struct Registry {
    nodes: HashMap<String, Node>,
}

impl Registry {
    pub fn new() -> Self { Self { nodes: HashMap::new() } }
    pub fn insert(&mut self, key: String, node: Node) { self.nodes.insert(key, node); }
}
"#;
        let file_path = tmp.join("src/registry.rs");
        fs::write(&file_path, content).unwrap();

        let result = parse_file(&file_path, &tmp, &Language::Rust).unwrap();
        assert_eq!(result.path, "src/registry.rs");
        assert_eq!(result.language, Language::Rust);
        assert_eq!(result.imports.len(), 2);
        assert!(result.symbols.iter().any(|s| s.name == "Registry" && s.kind == SymbolKind::Struct));
        assert!(result.symbols.iter().any(|s| s.name == "Registry" && s.kind == SymbolKind::Impl));

        let _ = fs::remove_dir_all(&tmp);
    }
}