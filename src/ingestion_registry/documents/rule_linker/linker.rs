//! linker.rs
//!
//! RuleLinker — finds connections between PDF rule nodes and codebase nodes.
//!
//! Two modes:
//!   Option A — similarity only (no LLM, fast, user choice)
//!   Option C — hybrid: similarity narrows candidates, LLM makes final call
//!
//! Output: Vec<RuleViolation> — each with rule text, code context,
//!         edge type (violates/implements/related), confidence, and LLM explanation.
//!
//! Edge types created in the graph:
//!   violates       — code does something the rule forbids
//!   implements     — code correctly follows the rule
//!   related_to_rule — code is relevant but compliance unclear

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Violates,
    Implements,
    RelatedToRule,
}

impl EdgeKind {
    pub fn as_label(&self) -> &'static str {
        match self {
            EdgeKind::Violates       => "violates",
            EdgeKind::Implements     => "implements",
            EdgeKind::RelatedToRule  => "related_to_rule",
        }
    }
}

/// One matched pair: a rule node connected to a code node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    /// Source URI of the PDF rule node e.g. `pdf://doc-uuid/page/3`
    pub rule_uri:       String,
    /// Short excerpt of the rule text (first 200 chars).
    pub rule_text:      String,
    /// Filename of the PDF this rule came from.
    pub rule_filename:  String,
    /// Source URI of the code node e.g. `codebase://github.com/owner/repo/src/server.rs`
    pub code_uri:       String,
    /// File path of the code node.
    pub code_path:      String,
    /// Symbol name if this is a function/struct node.
    pub code_symbol:    Option<String>,
    /// Cosine similarity between rule and code embeddings.
    pub similarity:     f32,
    /// Edge type determined by LLM (or heuristic in Option A).
    pub edge_kind:      EdgeKind,
    /// Confidence 0.0–1.0 (similarity score for Option A, LLM confidence for Option C).
    pub confidence:     f64,
    /// LLM explanation (empty string in Option A mode).
    pub explanation:    String,
}

/// Summary returned to the frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct LinkResult {
    pub document_id:    String,
    pub filename:       String,
    pub matches:        Vec<RuleMatch>,
    pub violates_count: usize,
    pub implements_count: usize,
    pub related_count:  usize,
    /// Nodes + edges for frontend graph rendering.
    pub graph_nodes:    Vec<LinkGraphNode>,
    pub graph_edges:    Vec<LinkGraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraphNode {
    pub id:     String,
    pub label:  String,
    pub kind:   String,   // "rule" | "code_file" | "code_symbol"
    pub source: String,   // "pdf" | "codebase"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraphEdge {
    pub from:        String,
    pub to:          String,
    pub label:       String,
    pub confidence:  f64,
}

/// Configuration for a link run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkConfig {
    /// Minimum cosine similarity to consider a pair as candidates.
    /// Default 0.65. User-configurable.
    #[serde(default = "default_threshold")]
    pub similarity_threshold: f32,

    /// Max code nodes to consider per rule node (similarity top-k).
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// Use LLM for final edge classification (Option C).
    /// If false, use similarity + heuristics only (Option A).
    #[serde(default = "default_use_llm")]
    pub use_llm: bool,

    /// Anthropic API key — required if use_llm is true.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Only link nodes from this document_id (empty = all PDF nodes).
    #[serde(default)]
    pub document_id_filter: Option<String>,

    /// Only link against codebase nodes whose path starts with this prefix.
    #[serde(default)]
    pub code_path_filter: Option<String>,
}

fn default_threshold() -> f32 { 0.65 }
fn default_top_k()     -> usize { 5 }
fn default_use_llm()   -> bool  { true }

// ── Core structs for graph access ─────────────────────────────────────────────

/// Minimal view of a graph node for the linker.
#[derive(Debug, Clone)]
pub struct NodeView {
    pub id:        String,   // NodeId as string
    pub uri:       String,
    pub text:      String,
    pub source:    String,   // "pdf" | "codebase"
    pub filename:  String,   // for pdf nodes
    pub doc_id:    String,   // for pdf nodes
    pub path:      String,   // for codebase nodes
    pub symbol:    Option<String>,
    pub embeddings: Vec<f32>,
}

// ── Linker ────────────────────────────────────────────────────────────────────

/// Extract rule nodes and code nodes from graph node metadata.
/// Returns (rule_nodes, code_nodes).
pub fn partition_nodes(nodes: &[NodeView]) -> (Vec<NodeView>, Vec<NodeView>) {
    let mut rules = Vec::new();
    let mut code  = Vec::new();
    for n in nodes {
        match n.source.as_str() {
            "pdf"      => rules.push(n.clone()),
            "codebase" => code.push(n.clone()),
            _          => {}
        }
    }
    (rules, code)
}

/// Cosine similarity between two embedding vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot:  f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na:   f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb:   f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Option A — similarity + heuristic edge classification. No LLM.
/// Returns candidate matches above the threshold.
pub fn link_by_similarity(
    rules:     &[NodeView],
    code:      &[NodeView],
    config:    &LinkConfig,
) -> Vec<RuleMatch> {
    let mut matches = Vec::new();

    for rule in rules {
        // Apply document_id filter.
        if let Some(ref filter) = config.document_id_filter {
            if !filter.is_empty() && rule.doc_id != *filter {
                continue;
            }
        }

        // Score all code nodes.
        let mut scored: Vec<(f32, &NodeView)> = code
            .iter()
            .filter(|c| {
                // Apply path filter.
                config.code_path_filter.as_ref()
                    .map(|f| c.path.starts_with(f.as_str()))
                    .unwrap_or(true)
            })
            .map(|c| (cosine_similarity(&rule.embeddings, &c.embeddings), c))
            .filter(|(sim, _)| *sim >= config.similarity_threshold)
            .collect();

        scored.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(config.top_k);

        for (sim, code_node) in scored {
            // Heuristic edge classification:
            // High similarity (>0.80) → likely implements or violates
            // Medium (0.65-0.80) → related_to_rule
            // We lean toward "related" in Option A since we can't verify intent.
            let edge_kind = if sim > 0.80 {
                // Check for negative keywords in rule text → violates
                let rule_lower = rule.text.to_lowercase();
                if rule_lower.contains("never")
                    || rule_lower.contains("must not")
                    || rule_lower.contains("avoid")
                    || rule_lower.contains("do not")
                    || rule_lower.contains("prohibited")
                    || rule_lower.contains("forbidden")
                {
                    EdgeKind::Violates
                } else {
                    EdgeKind::Implements
                }
            } else {
                EdgeKind::RelatedToRule
            };

            matches.push(RuleMatch {
                rule_uri:      rule.uri.clone(),
                rule_text:     rule.text.chars().take(200).collect(),
                rule_filename: rule.filename.clone(),
                code_uri:      code_node.uri.clone(),
                code_path:     code_node.path.clone(),
                code_symbol:   code_node.symbol.clone(),
                similarity:    sim,
                edge_kind,
                confidence:    sim as f64,
                explanation:   String::new(), // no LLM in Option A
            });
        }
    }

    // Sort by similarity descending.
    matches.sort_unstable_by(|a, b| {
        b.similarity.partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    matches
}

/// Option C — hybrid: similarity finds candidates, LLM classifies edge type and explains.
/// Falls back to Option A if api_key is missing.
pub async fn link_hybrid(
    rules:   &[NodeView],
    code:    &[NodeView],
    config:  &LinkConfig,
) -> Vec<RuleMatch> {
    // Step 1: get candidates via similarity.
    let candidates = link_by_similarity(rules, code, config);

    let api_key = match &config.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => return candidates, // fall back to Option A
    };

    // Step 2: LLM classification per candidate.
    let client = reqwest::Client::new();
    let mut results = Vec::with_capacity(candidates.len());

    for mut candidate in candidates {
        // Find the full code text for context.
        let code_text = code
            .iter()
            .find(|c| c.uri == candidate.code_uri)
            .map(|c| c.text.chars().take(500).collect::<String>())
            .unwrap_or_default();

        let prompt = format!(
            "You are a code review assistant analyzing whether code follows a rule.\n\n\
             RULE (from {filename}):\n{rule}\n\n\
             CODE ({path}):\n{code}\n\n\
             Classify the relationship:\n\
             - violates: the code clearly breaks this rule\n\
             - implements: the code correctly follows this rule\n\
             - related_to_rule: the code is relevant but compliance is unclear\n\n\
             Respond with JSON only, no markdown:\n\
             {{\"edge_kind\": \"violates\"|\"implements\"|\"related_to_rule\", \
               \"confidence\": 0.0-1.0, \
               \"explanation\": \"one sentence explanation\"}}",
            filename = candidate.rule_filename,
            rule     = candidate.rule_text,
            code     = code_text,
            path     = candidate.code_path,
        );

        let res = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-sonnet-4-20250514",
                "max_tokens": 200,
                "messages": [{ "role": "user", "content": prompt }]
            }))
            .send()
            .await;

        match res {
            Ok(resp) => {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    let text = body["content"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    // Strip markdown fences if present.
                    let clean = text
                        .trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(clean) {
                        if let Some(kind_str) = parsed["edge_kind"].as_str() {
                            candidate.edge_kind = match kind_str {
                                "violates"       => EdgeKind::Violates,
                                "implements"     => EdgeKind::Implements,
                                _                => EdgeKind::RelatedToRule,
                            };
                        }
                        if let Some(conf) = parsed["confidence"].as_f64() {
                            candidate.confidence = conf;
                        }
                        if let Some(exp) = parsed["explanation"].as_str() {
                            candidate.explanation = exp.to_string();
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[rule_linker] LLM call failed for {}: {e}", candidate.code_uri);
                // Keep the similarity-based classification.
            }
        }

        results.push(candidate);
    }

    results
}

/// Build the LinkResult summary from a list of matches.
pub fn build_result(
    document_id: &str,
    filename:    &str,
    matches:     Vec<RuleMatch>,
) -> LinkResult {
    let violates_count  = matches.iter().filter(|m| m.edge_kind == EdgeKind::Violates).count();
    let implements_count = matches.iter().filter(|m| m.edge_kind == EdgeKind::Implements).count();
    let related_count   = matches.iter().filter(|m| m.edge_kind == EdgeKind::RelatedToRule).count();

    // Build graph nodes + edges for frontend rendering.
    let mut node_map: HashMap<String, LinkGraphNode> = HashMap::new();
    let mut edges = Vec::new();

    for m in &matches {
        // Rule node.
        node_map.entry(m.rule_uri.clone()).or_insert(LinkGraphNode {
            id:     m.rule_uri.clone(),
            label:  m.rule_text.chars().take(60).collect(),
            kind:   "rule".to_string(),
            source: "pdf".to_string(),
        });

        // Code node.
        let code_label = m.code_symbol
            .as_deref()
            .unwrap_or_else(|| {
                m.code_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&m.code_path)
            })
            .to_string();

        let code_kind = if m.code_symbol.is_some() {
            "code_symbol"
        } else {
            "code_file"
        };

        node_map.entry(m.code_uri.clone()).or_insert(LinkGraphNode {
            id:     m.code_uri.clone(),
            label:  code_label,
            kind:   code_kind.to_string(),
            source: "codebase".to_string(),
        });

        edges.push(LinkGraphEdge {
            from:       m.rule_uri.clone(),
            to:         m.code_uri.clone(),
            label:      m.edge_kind.as_label().to_string(),
            confidence: m.confidence,
        });
    }

    let mut graph_nodes: Vec<LinkGraphNode> = node_map.into_values().collect();
    graph_nodes.sort_by(|a, b| a.id.cmp(&b.id));

    LinkResult {
        document_id:     document_id.to_string(),
        filename:        filename.to_string(),
        matches,
        violates_count,
        implements_count,
        related_count,
        graph_nodes,
        graph_edges: edges,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(
        id: &str, uri: &str, text: &str,
        source: &str, path: &str,
        embeddings: Vec<f32>,
    ) -> NodeView {
        NodeView {
            id:         id.to_string(),
            uri:        uri.to_string(),
            text:       text.to_string(),
            source:     source.to_string(),
            filename:   if source == "pdf" { "test.pdf".to_string() } else { String::new() },
            doc_id:     if source == "pdf" { "doc-1".to_string() } else { String::new() },
            path:       path.to_string(),
            symbol:     None,
            embeddings,
        }
    }

    fn unit_vec(dim: usize, hot: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; dim];
        v[hot] = 1.0;
        v
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let empty: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&empty, &empty), 0.0);
    }

    #[test]
    fn test_partition_nodes() {
        let nodes = vec![
            make_node("r1", "pdf://doc/page/1", "Never store passwords", "pdf", "", vec![1.0, 0.0]),
            make_node("c1", "codebase://repo/src/auth.rs", "pub fn login()", "codebase", "src/auth.rs", vec![0.9, 0.1]),
            make_node("c2", "codebase://repo/src/db.rs", "fn store()", "codebase", "src/db.rs", vec![0.1, 0.9]),
        ];
        let (rules, code) = partition_nodes(&nodes);
        assert_eq!(rules.len(), 1);
        assert_eq!(code.len(), 2);
    }

    #[test]
    fn test_link_by_similarity_above_threshold() {
        let rule = make_node(
            "r1", "pdf://doc/page/1",
            "Never store passwords in plaintext",
            "pdf", "", unit_vec(4, 0),
        );
        // High similarity code node.
        let code_match = make_node(
            "c1", "codebase://repo/src/auth.rs",
            "fn store_password(p: &str) { db.insert(p) }",
            "codebase", "src/auth.rs", unit_vec(4, 0),
        );
        // Low similarity code node.
        let code_miss = make_node(
            "c2", "codebase://repo/src/graph.rs",
            "pub struct Graph {}",
            "codebase", "src/graph.rs", unit_vec(4, 3),
        );

        let config = LinkConfig {
            similarity_threshold: 0.65,
            top_k:                5,
            use_llm:              false,
            api_key:              None,
            document_id_filter:   None,
            code_path_filter:     None,
        };

        let matches = link_by_similarity(
            &[rule],
            &[code_match, code_miss],
            &config,
        );

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].code_uri, "codebase://repo/src/auth.rs");
        assert!(matches[0].similarity >= 0.65);
    }

    #[test]
    fn test_link_by_similarity_below_threshold() {
        let rule = make_node(
            "r1", "pdf://doc/page/1", "Never store passwords",
            "pdf", "", unit_vec(4, 0),
        );
        let code = make_node(
            "c1", "codebase://repo/src/graph.rs", "pub struct Graph {}",
            "codebase", "src/graph.rs", unit_vec(4, 3),
        );
        let config = LinkConfig {
            similarity_threshold: 0.65,
            top_k: 5, use_llm: false,
            api_key: None,
            document_id_filter: None,
            code_path_filter: None,
        };
        let matches = link_by_similarity(&[rule], &[code], &config);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_heuristic_violates_keyword() {
        let rule = make_node(
            "r1", "pdf://doc/page/1",
            "Never expose API keys in client-side code",
            "pdf", "", unit_vec(4, 0),
        );
        let code = make_node(
            "c1", "codebase://repo/src/config.ts",
            "export const API_KEY = process.env.NEXT_PUBLIC_KEY",
            "codebase", "src/config.ts",
            // High similarity — same dimension
            vec![0.95, 0.05, 0.0, 0.0],
        );
        let config = LinkConfig {
            similarity_threshold: 0.60,
            top_k: 5, use_llm: false,
            api_key: None,
            document_id_filter: None,
            code_path_filter: None,
        };
        let matches = link_by_similarity(&[rule], &[code], &config);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].edge_kind, EdgeKind::Violates);
    }

    #[test]
    fn test_document_id_filter() {
        let rule_doc1 = NodeView {
            id: "r1".to_string(),
            uri: "pdf://doc-1/page/1".to_string(),
            text: "rule text".to_string(),
            source: "pdf".to_string(),
            filename: "owasp.pdf".to_string(),
            doc_id: "doc-1".to_string(),
            path: String::new(),
            symbol: None,
            embeddings: unit_vec(4, 0),
        };
        let rule_doc2 = NodeView {
            doc_id: "doc-2".to_string(),
            uri: "pdf://doc-2/page/1".to_string(),
            id: "r2".to_string(),
            ..rule_doc1.clone()
        };
        let code = make_node(
            "c1", "codebase://repo/src/auth.rs", "fn auth()",
            "codebase", "src/auth.rs", unit_vec(4, 0),
        );
        let config = LinkConfig {
            similarity_threshold: 0.5,
            top_k: 5, use_llm: false,
            api_key: None,
            document_id_filter: Some("doc-1".to_string()),
            code_path_filter: None,
        };
        let matches = link_by_similarity(&[rule_doc1, rule_doc2], &[code], &config);
        // Only doc-1 rule should match.
        assert!(matches.iter().all(|m| m.rule_uri.contains("doc-1")));
    }

    #[test]
    fn test_build_result_counts() {
        let matches = vec![
            RuleMatch {
                rule_uri: "pdf://doc/page/1".to_string(),
                rule_text: "rule 1".to_string(),
                rule_filename: "test.pdf".to_string(),
                code_uri: "codebase://repo/src/a.rs".to_string(),
                code_path: "src/a.rs".to_string(),
                code_symbol: None,
                similarity: 0.9,
                edge_kind: EdgeKind::Violates,
                confidence: 0.9,
                explanation: String::new(),
            },
            RuleMatch {
                rule_uri: "pdf://doc/page/2".to_string(),
                rule_text: "rule 2".to_string(),
                rule_filename: "test.pdf".to_string(),
                code_uri: "codebase://repo/src/b.rs".to_string(),
                code_path: "src/b.rs".to_string(),
                code_symbol: Some("my_fn".to_string()),
                similarity: 0.75,
                edge_kind: EdgeKind::Implements,
                confidence: 0.75,
                explanation: String::new(),
            },
            RuleMatch {
                rule_uri: "pdf://doc/page/3".to_string(),
                rule_text: "rule 3".to_string(),
                rule_filename: "test.pdf".to_string(),
                code_uri: "codebase://repo/src/c.rs".to_string(),
                code_path: "src/c.rs".to_string(),
                code_symbol: None,
                similarity: 0.68,
                edge_kind: EdgeKind::RelatedToRule,
                confidence: 0.68,
                explanation: "might be related".to_string(),
            },
        ];

        let result = build_result("doc-1", "test.pdf", matches);
        assert_eq!(result.violates_count, 1);
        assert_eq!(result.implements_count, 1);
        assert_eq!(result.related_count, 1);
        assert_eq!(result.graph_nodes.len(), 6); // 3 rule + 3 code
        assert_eq!(result.graph_edges.len(), 3);
    }

    #[test]
    fn test_edge_kind_labels() {
        assert_eq!(EdgeKind::Violates.as_label(),      "violates");
        assert_eq!(EdgeKind::Implements.as_label(),    "implements");
        assert_eq!(EdgeKind::RelatedToRule.as_label(), "related_to_rule");
    }

    #[test]
    fn test_path_filter() {
        let rule = make_node(
            "r1", "pdf://doc/page/1", "security rule",
            "pdf", "", unit_vec(4, 0),
        );
        let src_file = make_node(
            "c1", "codebase://repo/src/auth.rs", "fn auth()",
            "codebase", "src/auth.rs", unit_vec(4, 0),
        );
        let test_file = make_node(
            "c2", "codebase://repo/tests/auth_test.rs", "fn test()",
            "codebase", "tests/auth_test.rs", unit_vec(4, 0),
        );
        let config = LinkConfig {
            similarity_threshold: 0.5,
            top_k: 5, use_llm: false,
            api_key: None,
            document_id_filter: None,
            code_path_filter: Some("src/".to_string()),
        };
        let matches = link_by_similarity(&[rule], &[src_file, test_file], &config);
        // Only src/ files should match.
        assert!(matches.iter().all(|m| m.code_path.starts_with("src/")));
    }
}