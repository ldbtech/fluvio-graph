//! normalizer.rs
//!
//! Translates a `ParsedFile` into `NormalizedChunk`s for the codebase domain graph.
//!
//! Each file produces:
//!   1. One FILE chunk   — embeds the whole file's public surface + context
//!   2. One chunk per SYMBOL — embeds signature + context (file, kind, name)
//!
//! Pre-defined edges:
//!   file   → file    (import edges — resolved internal imports)
//!   file   → symbol  (contains edges — file owns this symbol)
//!   symbol → file    (defined_in — reverse of contains)
//!   symbol → symbol  (calls edges — function calls another function)
//!
//! Embeddable text format (signature + context):
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
/// `owner` and `repo` are the GitHub coordinates.
/// `start_index` is the chunk index offset so indices are globally unique within a batch.
pub fn normalize_file(
    parsed:      &ParsedFile,
    owner:       &str,
    repo:        &str,
    start_index: usize,
) -> Vec<NormalizedChunk> {
    let mut chunks = Vec::new();

    // ── 1. File-level chunk ───────────────────────────────────────────────────
    let file_chunk    = make_file_chunk(parsed, owner, repo, start_index);
    let this_file_uri = file_chunk.source_uri.clone();
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

        // File chunk gets a "contains" edge to every symbol.
        if let Some(fc) = chunks.first_mut() {
            fc.pre_defined_edges.push(PreDefinedEdge {
                to_uri:                  sym_uri,
                label:                   "contains".to_string(),
                relationship_probability: 1.0,
                token_cost:              0,
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
                    token_cost:              0,
                });
            }
        }
    }

    // ── 4. Calls edges between symbol chunks ──────────────────────────────────
    // Build a name → chunk_index map for fast lookup.
    // chunk 0 = file, chunk i+1 = symbol i.
    let sym_index: HashMap<&str, usize> = parsed
        .symbols
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i + 1))
        .collect();

    for (i, symbol) in parsed.symbols.iter().enumerate() {
        if symbol.calls.is_empty() {
            continue;
        }
        let caller_chunk_idx = i + 1;
        if caller_chunk_idx >= chunks.len() {
            continue;
        }

        for callee_name in &symbol.calls {
            // Look up callee in same file first.
            if let Some(&callee_chunk_idx) = sym_index.get(callee_name.as_str()) {
                if callee_chunk_idx == caller_chunk_idx || callee_chunk_idx >= chunks.len() {
                    continue;
                }
                let callee_uri = symbol_uri(owner, repo, &parsed.path, callee_name);
                if let Some(caller_chunk) = chunks.get_mut(caller_chunk_idx) {
                    caller_chunk.pre_defined_edges.push(PreDefinedEdge {
                        to_uri:                  callee_uri,
                        label:                   "calls".to_string(),
                        relationship_probability: 0.90,
                        token_cost:              1,
                    });
                }
            }
            // If callee not in same file it will be linked via cross-file
            // import resolution in the resolver — no action needed here.
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

    let public_symbols: Vec<String> = parsed
        .symbols
        .iter()
        .filter(|s| s.is_public)
        .map(|s| {
            let calls_str = if s.calls.is_empty() {
                String::new()
            } else {
                format!(" [calls: {}]", s.calls.join(", "))
            };
            format!(
                "  {} {}: {}{}",
                symbol_kind_label(&s.kind),
                s.name,
                s.signature,
                calls_str
            )
        })
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
    metadata.insert("symbol_count".to_string(), parsed.symbols.len().to_string());
    metadata.insert("import_count".to_string(), parsed.imports.len().to_string());
    metadata.insert(
        "public_symbols".to_string(),
        parsed.symbols.iter()
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

    // Include calls in the embeddable text so the LLM can reason about call relationships.
    let calls_line = if symbol.calls.is_empty() {
        String::new()
    } else {
        format!("\ncalls: {}", symbol.calls.join(", "))
    };

    let text = format!(
        "file: {path}\nrepo: {owner}/{repo}\nlanguage: {lang}\nkind: {kind}\nname: {name}\nsignature: {sig}{calls}",
        path  = parsed.path,
        lang  = format!("{:?}", parsed.language).to_lowercase(),
        kind  = symbol_kind_label(&symbol.kind),
        name  = symbol.name,
        sig   = symbol.signature,
        calls = calls_line,
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

    if !symbol.calls.is_empty() {
        metadata.insert("calls".to_string(), symbol.calls.join(","));
    }

    // Defined_in edge back to the parent file.
    let parent_edge = PreDefinedEdge {
        to_uri:                  file_uri.to_string(),
        label:                   "defined_in".to_string(),
        relationship_probability: 1.0,
        token_cost:              0,
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
                    resolved: None,
                    line:     2,
                },
            ],
            symbols: vec![
                Symbol {
                    name:      "Node".to_string(),
                    kind:      SymbolKind::Struct,
                    signature: "pub struct Node { pub id: NodeId }".to_string(),
                    line:      10,
                    is_public: true,
                    calls:     vec![],
                },
                Symbol {
                    name:      "add_node".to_string(),
                    kind:      SymbolKind::Function,
                    signature: "pub fn add_node(&mut self, node: Node) -> NodeId".to_string(),
                    line:      30,
                    is_public: true,
                    calls:     vec!["internal_helper".to_string()],
                },
                Symbol {
                    name:      "internal_helper".to_string(),
                    kind:      SymbolKind::Function,
                    signature: "fn internal_helper() -> bool".to_string(),
                    line:      50,
                    is_public: false,
                    calls:     vec![],
                },
            ],
        }
    }

    #[test]
    fn test_normalize_file_chunk_count() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        assert_eq!(chunks.len(), 4); // 1 file + 3 symbols
    }

    #[test]
    fn test_file_chunk_is_first() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        let fc = &chunks[0];
        assert_eq!(fc.metadata.get("kind").unwrap(), "file");
        assert_eq!(
            fc.source_uri,
            "codebase://github.com/ldbtech/FluvioGraph/src/graph/structs.rs"
        );
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
        assert!(text.contains("use crate::graph::enums::Domain"));
        assert!(!text.contains("use std::collections::HashMap"));
    }

    #[test]
    fn test_symbol_chunks_have_context() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        let node_chunk = &chunks[1];
        assert!(node_chunk.text.contains("kind: struct"));
        assert!(node_chunk.text.contains("name: Node"));
        assert!(node_chunk.text.contains("file: src/graph/structs.rs"));
        assert_eq!(
            node_chunk.source_uri,
            "codebase://github.com/ldbtech/FluvioGraph/src/graph/structs.rs#Node"
        );
    }

    #[test]
    fn test_symbol_chunk_metadata() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        let chunk = &chunks[2]; // add_node
        assert_eq!(chunk.metadata.get("name").unwrap(), "add_node");
        assert_eq!(chunk.metadata.get("kind").unwrap(), "function");
        assert_eq!(chunk.metadata.get("is_public").unwrap(), "true");
        assert_eq!(chunk.metadata.get("line").unwrap(), "30");
        // calls stored in metadata
        assert_eq!(chunk.metadata.get("calls").unwrap(), "internal_helper");
    }

    #[test]
    fn test_calls_in_symbol_text() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        let add_node = &chunks[2];
        assert!(add_node.text.contains("calls: internal_helper"),
            "expected calls in text, got: {}", add_node.text);
    }

    #[test]
    fn test_calls_edge_created() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        // add_node (chunk 2) should have a "calls" edge to internal_helper (chunk 3)
        let add_node = &chunks[2];
        let calls_edges: Vec<_> = add_node.pre_defined_edges.iter()
            .filter(|e| e.label == "calls")
            .collect();
        assert_eq!(calls_edges.len(), 1);
        assert!(calls_edges[0].to_uri.ends_with("#internal_helper"));
        assert_eq!(calls_edges[0].relationship_probability, 0.90);
    }

    #[test]
    fn test_no_calls_edge_when_empty() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        // Node struct (chunk 1) has no calls
        let node_chunk = &chunks[1];
        let calls_edges: Vec<_> = node_chunk.pre_defined_edges.iter()
            .filter(|e| e.label == "calls")
            .collect();
        assert!(calls_edges.is_empty());
    }

    #[test]
    fn test_import_edges_on_file_chunk() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        let import_edges: Vec<_> = chunks[0].pre_defined_edges.iter()
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
        let contains_edges: Vec<_> = chunks[0].pre_defined_edges.iter()
            .filter(|e| e.label == "contains")
            .collect();
        assert_eq!(contains_edges.len(), 3);
        assert!(contains_edges.iter().any(|e| e.to_uri.ends_with("#Node")));
        assert!(contains_edges.iter().any(|e| e.to_uri.ends_with("#add_node")));
        assert!(contains_edges.iter().any(|e| e.to_uri.ends_with("#internal_helper")));
    }

    #[test]
    fn test_symbol_defined_in_edge() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        for chunk in chunks.iter().skip(1) {
            let defined_in: Vec<_> = chunk.pre_defined_edges.iter()
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
    fn test_chunk_indices_sequential() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 10);
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
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].pre_defined_edges.is_empty());
    }

    #[test]
    fn test_source_uris_unique() {
        let parsed = make_parsed_file();
        let chunks = normalize_file(&parsed, "ldbtech", "FluvioGraph", 0);
        let uris: Vec<&str> = chunks.iter().map(|c| c.source_uri.as_str()).collect();
        let unique: std::collections::HashSet<&str> = uris.iter().copied().collect();
        assert_eq!(uris.len(), unique.len());
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
                calls:     vec!["save_token".to_string()],
            }],
        };
        let chunks = normalize_file(&parsed, "owner", "repo", 0);
        assert_eq!(chunks.len(), 2);
        let sym = &chunks[1];
        assert!(sym.text.contains("language: python"));
        assert!(sym.text.contains("kind: function"));
        assert!(sym.text.contains("name: exchange_code"));
        assert!(sym.text.contains("calls: save_token"));
    }
}