//! normalizer.rs
//!
//! Translates a `ParsedFile` into `NormalizedChunk`s for the codebase domain graph.
//!
//! Each file produces:
//!   1. One FILE chunk   — embeds the whole file's public surface + context
//!   2. One chunk per SYMBOL — embeds signature + context (file, kind, name)
//!
//! Pre-defined edges:
//!   file  → file    (import edges — resolved internal imports)
//!   file  → symbol  (contains edges — file owns this symbol)
//!
//! Embeddable text format (Option B — signature + context):
//!   file:     src/graph/structs.rs
//!   language: rust
//!   kind:     function
//!   name:     add_node
//!   signature: pub fn add_node(&mut self, node: Node) -> NodeId

use std::collections::HashMap;

use crate::graph::enums::Domain;
use crate::ingestion_registry::connector::{NormalizedChunk, PreDefinedEdge};
use super::parser::{ParsedFile, Symbol, SymbolKind};

// ── Source URI helpers ────────────────────────────────────────────────────────

/// `codebase://github.com/owner/repo/src/graph/structs.rs`
pub fn file_uri(owner: &str, repo: &str, path: &str) -> String {
    format!("codebase://github.com/{owner}/{repo}/{path}")
}

/// `codebase://github.com/owner/repo/src/graph/structs.rs#add_node`
pub fn symbol_uri(owner: &str, repo: &str, path: &str, symbol_name: &str) -> String {
    format!("codebase://github.com/{owner}/{repo}/{path}#{symbol_name}")
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// Normalize a `ParsedFile` into a flat list of `NormalizedChunk`s.
///
/// `owner` and `repo` are the GitHub coordinates — used to build stable source URIs.
/// `start_index` is the chunk index offset (so indices are globally unique within a sync batch).
pub fn normalize_file(
    parsed:      &ParsedFile,
    owner:       &str,
    repo:        &str,
    start_index: usize,
) -> Vec<NormalizedChunk> {
    let mut chunks = Vec::new();

    // ── 1. File-level chunk ───────────────────────────────────────────────────
    let file_chunk      = make_file_chunk(parsed, owner, repo, start_index);
    let this_file_uri   = file_chunk.source_uri.clone();
    chunks.push(file_chunk);

    // ── 2. Symbol chunks ──────────────────────────────────────────────────────
    for (i, symbol) in parsed.symbols.iter().enumerate() {
        let sym_uri   = symbol_uri(owner, repo, &parsed.path, &symbol.name);
        let sym_chunk = make_symbol_chunk(
            parsed,
            symbol,
            owner,
            repo,
            start_index + 1 + i,
            &this_file_uri,
        );
        chunks.push(sym_chunk);

        // Back-reference: file chunk gets a "contains" edge to this symbol.
        if let Some(fc) = chunks.first_mut() {
            fc.pre_defined_edges.push(PreDefinedEdge {
                to_uri:                  sym_uri,
                label:                   "contains".to_string(),
                relationship_probability: 1.0,
                token_cost:              1,
            });
        }
    }

    // ── 3. Import edges on file chunk ─────────────────────────────────────────
    for import in &parsed.imports {
        if let Some(resolved_path) = &import.resolved {
            let target_uri = file_uri(owner, repo, resolved_path);
            if let Some(fc) = chunks.first_mut() {
                fc.pre_defined_edges.push(PreDefinedEdge {
                    to_uri:                  target_uri,
                    label:                   "imports".to_string(),
                    relationship_probability: 0.95,
                    token_cost:              2,
                });
            }
        }
    }

    chunks
}

// ── Chunk builders ────────────────────────────────────────────────────────────

fn make_file_chunk(
    parsed:      &ParsedFile,
    owner:       &str,
    repo:        &str,
    chunk_index: usize,
) -> NormalizedChunk {
    let source_uri = file_uri(owner, repo, &parsed.path);

    // Build embeddable text: file context + public surface summary.
    let public_symbols: Vec<String> = parsed
        .symbols
        .iter()
        .filter(|s| s.is_public)
        .map(|s| format!("  {} {}: {}", symbol_kind_label(&s.kind), s.name, s.signature))
        .collect();

    let internal_imports: Vec<String> = parsed
        .imports
        .iter()
        .filter(|i| i.resolved.is_some())
        .map(|i| format!("  {}", i.raw))
        .collect();

    let mut text_parts = vec![
        format!("file: {}", parsed.path),
        format!("repo: {owner}/{repo}"),
        format!("language: {:?}", parsed.language).to_lowercase(),
    ];

    if !internal_imports.is_empty() {
        text_parts.push(format!("imports:\n{}", internal_imports.join("\n")));
    }

    if !public_symbols.is_empty() {
        text_parts.push(format!("public api:\n{}", public_symbols.join("\n")));
    }

    let text = text_parts.join("\n");

    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(),    "codebase".to_string());
    metadata.insert("owner".to_string(),     owner.to_string());
    metadata.insert("repo".to_string(),      repo.to_string());
    metadata.insert("path".to_string(),      parsed.path.clone());
    metadata.insert("language".to_string(),  format!("{:?}", parsed.language).to_lowercase());
    metadata.insert("kind".to_string(),      "file".to_string());
    metadata.insert(
        "symbol_count".to_string(),
        parsed.symbols.len().to_string(),
    );
    metadata.insert(
        "import_count".to_string(),
        parsed.imports.len().to_string(),
    );
    metadata.insert(
        "public_symbols".to_string(),
        parsed
            .symbols
            .iter()
            .filter(|s| s.is_public)
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(","),
    );

    NormalizedChunk {
        text,
        metadata,
        chunk_index,
        source_uri,
        domain: Domain::Codebase,
        pre_defined_edges: vec![],
    }
}

fn make_symbol_chunk(
    parsed:      &ParsedFile,
    symbol:      &Symbol,
    owner:       &str,
    repo:        &str,
    chunk_index: usize,
    file_uri:    &str,
) -> NormalizedChunk {
    let source_uri = symbol_uri(owner, repo, &parsed.path, &symbol.name);

    // Embeddable text: signature + full context.
    let text = format!(
        "file: {path}\nrepo: {owner}/{repo}\nlanguage: {lang}\nkind: {kind}\nname: {name}\nsignature: {sig}",
        path  = parsed.path,
        lang  = format!("{:?}", parsed.language).to_lowercase(),
        kind  = symbol_kind_label(&symbol.kind),
        name  = symbol.name,
        sig   = symbol.signature,
    );

    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(),    "codebase".to_string());
    metadata.insert("owner".to_string(),     owner.to_string());
    metadata.insert("repo".to_string(),      repo.to_string());
    metadata.insert("path".to_string(),      parsed.path.clone());
    metadata.insert("language".to_string(),  format!("{:?}", parsed.language).to_lowercase());
    metadata.insert("kind".to_string(),      symbol_kind_label(&symbol.kind).to_string());
    metadata.insert("name".to_string(),      symbol.name.clone());
    metadata.insert("line".to_string(),      symbol.line.to_string());
    metadata.insert("is_public".to_string(), symbol.is_public.to_string());
    metadata.insert("signature".to_string(), symbol.signature.clone());

    // Symbol chunk points back to its parent file.
    let parent_edge = PreDefinedEdge {
        to_uri:                  file_uri.to_string(),
        label:                   "defined_in".to_string(),
        relationship_probability: 1.0,
        token_cost:              1,
    };

    NormalizedChunk {
        text,
        metadata,
        chunk_index,
        source_uri,
        domain: Domain::Codebase,
        pre_defined_edges: vec![parent_edge],
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn symbol_kind_label(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function  => "function",
        SymbolKind::Struct    => "struct",
        SymbolKind::Enum      => "enum",
        SymbolKind::Impl      => "impl",
        SymbolKind::Trait     => "trait",
        SymbolKind::Class     => "class",
        SymbolKind::Method    => "method",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Interface => "interface",
        SymbolKind::Constant  => "const",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion_registry::codebase::parser::{Import, Symbol, SymbolKind};
    use crate::ingestion_registry::codebase::tree::Language;

    fn make_parsed_file() -> ParsedFile {
        ParsedFile {
            path:     "src/graph/structs.rs".to_string(),
            language: Language::Rust,
            imports: vec![
                Import {
                    raw:      "use crate::graph::enums::Domain".to_string(),
                    resolved: Some("src/graph/enums.rs".to_string()),
                    line:     1,
                },
                Import {
                    raw:      "use std::collections::HashMap".to_string(),
                    resolved: None, // external
                    line:     2,
                },
            ],
            symbols: vec![
                Symbol {
                    name:      "Node".to_string(),
                    kind:      SymbolKind::Struct,
                    signature: "pub struct Node { pub id: NodeId, pub embeddings: Vec<f32> }"
                        .to_string(),
                    line:      10,
                    is_public: true,
                },
                Symbol {
                    name:      "add_node".to_string(),
                    kind:      SymbolKind::Function,
                    signature: "pub fn add_node(&mut self, node: Node) -> NodeId".to_string(),
                    line:      30,
                    is_public: true,
                },
                Symbol {
                    name:      "internal_helper".to_string(),
                    kind:      SymbolKind::Function,
                    signature: "fn internal_helper() -> bool".to_string(),
                    line:      50,
                    is_public: false,
                },
            ],
        }
    }

    #[test]
    fn test_normalize_file_chunk_count() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        // 1 file chunk + 3 symbol chunks = 4 total.
        assert_eq!(chunks.len(), 4);
    }

    #[test]
    fn test_file_chunk_is_first() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let file_chunk = &chunks[0];
        assert_eq!(file_chunk.metadata.get("kind").unwrap(), "file");
        assert_eq!(file_chunk.source_uri, "codebase://github.com/ldbtech/FluvioGraph/src/graph/structs.rs");
    }

    #[test]
    fn test_file_chunk_text_contains_context() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let text = &chunks[0].text;
        assert!(text.contains("file: src/graph/structs.rs"));
        assert!(text.contains("repo: ldbtech/FluvioGraph"));
        assert!(text.contains("language: rust"));
        assert!(text.contains("public api:"));
        assert!(text.contains("add_node"));
        assert!(text.contains("imports:"));
        // Internal import shows, external doesn't.
        assert!(text.contains("use crate::graph::enums::Domain"));
        assert!(!text.contains("use std::collections::HashMap"));
    }

    #[test]
    fn test_symbol_chunks_have_context() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        // Second chunk is the Node struct.
        let node_chunk = &chunks[1];
        assert!(node_chunk.text.contains("kind: struct"));
        assert!(node_chunk.text.contains("name: Node"));
        assert!(node_chunk.text.contains("file: src/graph/structs.rs"));
        assert!(node_chunk.text.contains("repo: ldbtech/FluvioGraph"));
        assert_eq!(
            node_chunk.source_uri,
            "codebase://github.com/ldbtech/FluvioGraph/src/graph/structs.rs#Node"
        );
    }

    #[test]
    fn test_symbol_chunk_metadata() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let add_node_chunk = &chunks[2]; // add_node is second symbol
        assert_eq!(add_node_chunk.metadata.get("name").unwrap(), "add_node");
        assert_eq!(add_node_chunk.metadata.get("kind").unwrap(), "function");
        assert_eq!(add_node_chunk.metadata.get("is_public").unwrap(), "true");
        assert_eq!(add_node_chunk.metadata.get("line").unwrap(), "30");
        assert_eq!(add_node_chunk.metadata.get("language").unwrap(), "rust");
    }

    #[test]
    fn test_import_edges_on_file_chunk() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let file_chunk = &chunks[0];
        // Should have: 3 "contains" edges + 1 "imports" edge (only resolved import).
        let import_edges: Vec<_> = file_chunk
            .pre_defined_edges
            .iter()
            .filter(|e| e.label == "imports")
            .collect();
        assert_eq!(import_edges.len(), 1);
        assert_eq!(
            import_edges[0].to_uri,
            "codebase://github.com/ldbtech/FluvioGraph/src/graph/enums.rs"
        );
    }

    #[test]
    fn test_contains_edges_on_file_chunk() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let file_chunk = &chunks[0];
        let contains_edges: Vec<_> = file_chunk
            .pre_defined_edges
            .iter()
            .filter(|e| e.label == "contains")
            .collect();
        // 3 symbols → 3 contains edges.
        assert_eq!(contains_edges.len(), 3);
        assert!(contains_edges
            .iter()
            .any(|e| e.to_uri.ends_with("#Node")));
        assert!(contains_edges
            .iter()
            .any(|e| e.to_uri.ends_with("#add_node")));
        assert!(contains_edges
            .iter()
            .any(|e| e.to_uri.ends_with("#internal_helper")));
    }

    #[test]
    fn test_symbol_defined_in_edge() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        // Every symbol chunk should have a "defined_in" edge back to the file.
        for chunk in chunks.iter().skip(1) {
            let defined_in: Vec<_> = chunk
                .pre_defined_edges
                .iter()
                .filter(|e| e.label == "defined_in")
                .collect();
            assert_eq!(defined_in.len(), 1);
            assert_eq!(
                defined_in[0].to_uri,
                "codebase://github.com/ldbtech/FluvioGraph/src/graph/structs.rs"
            );
        }
    }

    #[test]
    fn test_chunk_indices_are_sequential() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 10);

        // Start index was 10, so chunks should be 10, 11, 12, 13.
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, 10 + i);
        }
    }

    #[test]
    fn test_file_chunk_metadata() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let meta = &chunks[0].metadata;
        assert_eq!(meta.get("source").unwrap(), "codebase");
        assert_eq!(meta.get("owner").unwrap(), "ldbtech");
        assert_eq!(meta.get("repo").unwrap(), "FluvioGraph");
        assert_eq!(meta.get("symbol_count").unwrap(), "3");
        assert_eq!(meta.get("import_count").unwrap(), "2");
        // public_symbols should only contain public ones.
        let pub_syms = meta.get("public_symbols").unwrap();
        assert!(pub_syms.contains("Node"));
        assert!(pub_syms.contains("add_node"));
        assert!(!pub_syms.contains("internal_helper"));
    }

    #[test]
    fn test_empty_file_produces_one_chunk() {
        let parsed = ParsedFile {
            path:     "src/empty.rs".to_string(),
            language: Language::Rust,
            imports:  vec![],
            symbols:  vec![],
        };
        let chunks = normalize_file(&parsed, "owner", "repo", 0);
        assert_eq!(chunks.len(), 1); // just the file chunk
        assert!(chunks[0].pre_defined_edges.is_empty());
    }

    #[test]
    fn test_source_uris_are_unique() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);

        let uris: Vec<&str> = chunks.iter().map(|c| c.source_uri.as_str()).collect();
        let unique: std::collections::HashSet<&str> = uris.iter().copied().collect();
        assert_eq!(uris.len(), unique.len(), "all source URIs should be unique");
    }

    #[test]
    fn test_python_file_normalization() {
        let parsed = ParsedFile {
            path:     "auth/oauth.py".to_string(),
            language: Language::Python,
            imports: vec![Import {
                raw:      "from .token_store import save_token".to_string(),
                resolved: Some("auth/token_store.py".to_string()),
                line:     1,
            }],
            symbols: vec![Symbol {
                name:      "exchange_code".to_string(),
                kind:      SymbolKind::Function,
                signature: "async def exchange_code(code: str) -> GmailToken:".to_string(),
                line:      15,
                is_public: true,
            }],
        };

        let chunks = normalize_file(&parsed, "owner", "repo", 0);
        assert_eq!(chunks.len(), 2); // file + 1 symbol

        let sym = &chunks[1];
        assert!(sym.text.contains("language: python"));
        assert!(sym.text.contains("kind: function"));
        assert!(sym.text.contains("name: exchange_code"));
    }
}