//! security_agent.rs
//!
//! SecurityAgent — an LLM-powered agent that analyzes codebase nodes
//! against PDF security rules already in the knowledge graph.
//!
//! The agent:
//!   1. Reads all codebase nodes currently in the graph
//!   2. For each file node, finds relevant PDF rule nodes via similarity search
//!   3. Asks Claude to reason about whether the code violates any rules
//!   4. Writes violates/implements/related_to_rule edges into the graph
//!   5. Creates an Agent node with edges to everything it analyzed (audit trail)
//!   6. Returns a structured report
//!
//! Runs as a background tokio task — poll /agents/security/:id/status for progress.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::graph::enums::{Domain, NodeKind};
use crate::graph::fluvio_graph::FluvioGraph;
use crate::graph::structs::{DomainGraph, Edge, EdgeId, Node, NodeId};
use crate::graph::EmbeddingContext;

// ── Progress tracking ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentPhase {
    Idle,
    Initializing,
    Scanning,
    Analyzing,
    WritingEdges,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProgress {
    pub agent_id:       String,
    pub phase:          AgentPhase,
    pub current_file:   Option<String>,
    pub files_done:     usize,
    pub files_total:    usize,
    pub violations:     usize,
    pub error:          Option<String>,
    pub running:        bool,
}

impl AgentProgress {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id:     agent_id.to_string(),
            phase:        AgentPhase::Idle,
            current_file: None,
            files_done:   0,
            files_total:  0,
            violations:   0,
            error:        None,
            running:      false,
        }
    }

    pub fn percent(&self) -> u8 {
        if self.files_total == 0 { return 0; }
        ((self.files_done as f64 / self.files_total as f64) * 100.0) as u8
    }
}

/// Thread-safe progress state shared between the agent task and the HTTP handler.
#[derive(Debug)]
pub struct SecurityAgentProgress {
    inner: Mutex<AgentProgress>,
}

impl SecurityAgentProgress {
    pub fn new(agent_id: &str) -> Self {
        Self { inner: Mutex::new(AgentProgress::new(agent_id)) }
    }

    pub fn snapshot(&self) -> AgentProgress {
        self.inner.lock().unwrap().clone()
    }

    fn update<F: FnOnce(&mut AgentProgress)>(&self, f: F) {
        f(&mut self.inner.lock().unwrap());
    }
}

// ── Violation ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityViolation {
    pub file_path:      String,
    pub symbol:         Option<String>,
    pub code_uri:       String,
    pub rule_uri:       String,
    pub rule_text:      String,
    pub rule_source:    String,
    pub edge_kind:      String,   // "violates" | "implements" | "related_to_rule"
    pub confidence:     f64,
    pub explanation:    String,
    pub severity:       String,   // "critical" | "high" | "medium" | "low"
}

// ── Agent config ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAgentConfig {
    /// Only analyze codebase nodes whose path starts with this prefix.
    /// Empty = all codebase nodes in the graph.
    #[serde(default)]
    pub scope: Option<String>,

    /// Only use PDF nodes from this document_id.
    /// Empty = all PDF nodes in the graph.
    #[serde(default)]
    pub pdf_document_ids: Vec<String>,

    /// Minimum similarity to consider a PDF rule relevant to a code node.
    #[serde(default = "default_sim_threshold")]
    pub similarity_threshold: f32,

    /// Max PDF rules to consider per code file (top-k by similarity).
    #[serde(default = "default_top_k")]
    pub top_k_rules: usize,

    /// Max codebase files to analyze per run (safety cap).
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_sim_threshold() -> f32  { 0.55 }
fn default_top_k()          -> usize { 5 }
fn default_max_files()      -> usize { 100 }

impl Default for SecurityAgentConfig {
    fn default() -> Self {
        Self {
            scope:                None,
            pdf_document_ids:     vec![],
            similarity_threshold: default_sim_threshold(),
            top_k_rules:          default_top_k(),
            max_files:            default_max_files(),
        }
    }
}

// ── Agent result ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAgentResult {
    pub agent_id:         String,
    pub agent_node_id:    String,   // NodeId of the agent's own graph node
    pub files_analyzed:   usize,
    pub rules_checked:    usize,
    pub violations:       Vec<SecurityViolation>,
    pub violates_count:   usize,
    pub implements_count: usize,
    pub related_count:    usize,
    pub edges_written:    usize,
}

// ── Core agent logic ──────────────────────────────────────────────────────────

/// Snapshot of a graph node for the agent to work with.
#[derive(Clone)]
struct NodeSnapshot {
    id:         NodeId,
    uri:        String,
    text:       String,
    source:     String,
    path:       String,
    symbol:     Option<String>,
    filename:   String,
    doc_id:     String,
    embeddings: Vec<f32>,
}

fn snapshot_nodes(graph: &DomainGraph) -> Vec<NodeSnapshot> {
    graph.nodes.values().map(|n| NodeSnapshot {
        id:         n.id,
        uri:        n.source_uri.clone(),
        text:       n.source_text.clone(),
        source:     n.metadata.get("source").cloned().unwrap_or_default(),
        path:       n.metadata.get("path").cloned().unwrap_or_default(),
        symbol:     n.metadata.get("name").cloned(),
        filename:   n.metadata.get("filename").cloned().unwrap_or_default(),
        doc_id:     n.metadata.get("document_id").cloned().unwrap_or_default(),
        embeddings: n.embeddings.clone(),
    }).collect()
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32  = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na:  f32  = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb:  f32  = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Run the security agent as a background task.
/// Returns the full result when done.
pub async fn run_agent(
    agent_id:   String,
    config:     SecurityAgentConfig,
    api_key:    String,
    graph:      Arc<Mutex<DomainGraph>>,
    embed_ctx:  Arc<Mutex<EmbeddingContext>>,
    progress:   Arc<SecurityAgentProgress>,
    persist_fn: fn(&DomainGraph) -> anyhow::Result<()>,
) -> SecurityAgentResult {
    progress.update(|p| {
        p.running = true;
        p.phase   = AgentPhase::Initializing;
    });

    // ── 1. Snapshot nodes from graph ──────────────────────────────────────────
    let all_nodes = {
        let g = graph.lock().unwrap();
        snapshot_nodes(&g)
    };

    // Partition into code nodes and PDF rule nodes.
    let code_nodes: Vec<&NodeSnapshot> = all_nodes.iter()
        .filter(|n| n.source == "codebase")
        .filter(|n| n.metadata_path_matches(&config.scope))
        // Only file-level nodes (not symbol chunks) to avoid redundancy.
        .filter(|n| !n.uri.contains('#'))
        .take(config.max_files)
        .collect();

    let rule_nodes: Vec<&NodeSnapshot> = all_nodes.iter()
        .filter(|n| n.source == "pdf")
        .filter(|n| {
            config.pdf_document_ids.is_empty()
                || config.pdf_document_ids.contains(&n.doc_id)
        })
        .collect();

    let files_total = code_nodes.len();

    progress.update(|p| {
        p.phase       = AgentPhase::Scanning;
        p.files_total = files_total;
    });

    if code_nodes.is_empty() || rule_nodes.is_empty() {
        progress.update(|p| {
            p.running = false;
            p.phase   = AgentPhase::Done;
            p.error   = Some(if code_nodes.is_empty() {
                "no codebase nodes in graph — ingest a repo first".to_string()
            } else {
                "no PDF rule nodes in graph — upload a security PDF first".to_string()
            });
        });
        return SecurityAgentResult {
            agent_id,
            agent_node_id:    String::new(),
            files_analyzed:   0,
            rules_checked:    0,
            violations:       vec![],
            violates_count:   0,
            implements_count: 0,
            related_count:    0,
            edges_written:    0,
        };
    }

    // ── 2. Create agent node in the graph ─────────────────────────────────────
    let agent_node_id = {
        let mut g = graph.lock().unwrap();
        create_agent_node(&mut g, &agent_id, &config)
    };

    let http_client  = reqwest::Client::new();
    let mut violations:     Vec<SecurityViolation> = Vec::new();
    let mut rules_checked   = 0usize;
    let mut edges_to_write: Vec<(NodeId, NodeId, String, f64)> = Vec::new();

    // ── 3. Analyze each code file ─────────────────────────────────────────────
    for (i, code_node) in code_nodes.iter().enumerate() {
        progress.update(|p| {
            p.phase        = AgentPhase::Analyzing;
            p.files_done   = i;
            p.current_file = Some(code_node.path.clone());
        });

        // Find top-k most relevant PDF rules by similarity.
        let mut scored: Vec<(f32, &NodeSnapshot)> = rule_nodes.iter()
            .map(|r| (cosine_sim(&code_node.embeddings, &r.embeddings), *r))
            .filter(|(sim, _)| *sim >= config.similarity_threshold)
            .collect();

        scored.sort_unstable_by(|a, b|
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
        );
        scored.truncate(config.top_k_rules);

        if scored.is_empty() {
            continue;
        }

        rules_checked += scored.len();

        // Build context for LLM.
        let rules_context = scored.iter()
            .enumerate()
            .map(|(i, (sim, rule))| format!(
                "Rule {} (from {}, similarity {:.2}):\n{}",
                i + 1,
                rule.filename,
                sim,
                rule.text.chars().take(300).collect::<String>()
            ))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "You are a security code reviewer. Analyze this code against the security rules below.\n\n\
             CODE FILE: {path}\n\
             {code}\n\n\
             SECURITY RULES:\n{rules}\n\n\
             For each rule that applies to this code, respond with a JSON array.\n\
             Each item: {{\"rule_index\": 1, \"edge_kind\": \"violates\"|\"implements\"|\"related_to_rule\", \
             \"confidence\": 0.0-1.0, \"severity\": \"critical\"|\"high\"|\"medium\"|\"low\", \
             \"explanation\": \"one sentence\"}}\n\
             Only include rules that are clearly relevant. If none apply, return [].\n\
             Respond with JSON array only, no markdown.",
            path  = code_node.path,
            code  = code_node.text.chars().take(800).collect::<String>(),
            rules = rules_context,
        );

        let res = http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": "claude-sonnet-4-20250514",
                "max_tokens": 800,
                "messages": [{ "role": "user", "content": prompt }]
            }))
            .send()
            .await;

        let llm_items = match res {
            Err(e) => {
                eprintln!("[security_agent] LLM error for {}: {e}", code_node.path);
                continue;
            }
            Ok(resp) => {
                match resp.json::<serde_json::Value>().await {
                    Err(e) => {
                        eprintln!("[security_agent] parse error: {e}");
                        continue;
                    }
                    Ok(body) => {
                        let text = body["content"][0]["text"]
                            .as_str()
                            .unwrap_or("[]")
                            .trim()
                            .trim_start_matches("```json")
                            .trim_start_matches("```")
                            .trim_end_matches("```")
                            .trim()
                            .to_string();

                        serde_json::from_str::<Vec<serde_json::Value>>(&text)
                            .unwrap_or_default()
                    }
                }
            }
        };

        // Process LLM response items.
        for item in &llm_items {
            let rule_idx = item["rule_index"].as_u64().unwrap_or(1) as usize;
            let rule_idx = rule_idx.saturating_sub(1).min(scored.len().saturating_sub(1));

            let (_, rule_node) = &scored[rule_idx];
            let edge_kind  = item["edge_kind"].as_str().unwrap_or("related_to_rule");
            let confidence = item["confidence"].as_f64().unwrap_or(0.5);
            let severity   = item["severity"].as_str().unwrap_or("medium").to_string();
            let explanation = item["explanation"].as_str().unwrap_or("").to_string();

            if edge_kind == "violates" || edge_kind == "implements" || edge_kind == "related_to_rule" {
                violations.push(SecurityViolation {
                    file_path:   code_node.path.clone(),
                    symbol:      code_node.symbol.clone(),
                    code_uri:    code_node.uri.clone(),
                    rule_uri:    rule_node.uri.clone(),
                    rule_text:   rule_node.text.chars().take(200).collect(),
                    rule_source: rule_node.filename.clone(),
                    edge_kind:   edge_kind.to_string(),
                    confidence,
                    explanation,
                    severity,
                });

                // Queue edge for writing.
                edges_to_write.push((
                    code_node.id,
                    rule_node.id,
                    edge_kind.to_string(),
                    confidence,
                ));

                if edge_kind == "violates" {
                    progress.update(|p| p.violations += 1);
                }
            }
        }
    }

    // ── 4. Write edges into the graph ─────────────────────────────────────────
    progress.update(|p| p.phase = AgentPhase::WritingEdges);

    let edges_written = {
        let mut g = graph.lock().unwrap();
        let mut count = 0usize;

        for (code_id, rule_id, label, confidence) in &edges_to_write {
            // Avoid duplicate edges.
            let dup = g.adj.get(code_id)
                .map(|edges| edges.iter().any(|e| e.to == *rule_id && e.label == *label))
                .unwrap_or(false);

            if !dup {
                let edge = Edge {
                    id:                       EdgeId::new(),
                    from:                     *code_id,
                    to:                       *rule_id,
                    token:                    ((1.0 - confidence) * 1000.0) as i32,
                    relationship_probability: *confidence,
                    label:                    label.clone(),
                    metadata:                 HashMap::new(),
                };
                if g.insert_edge(edge).is_ok() {
                    count += 1;
                }
            }

            // Add "analyzed_by" edge from code node to agent node.
            if let Some(agent_id_parsed) = NodeId::try_from_str(&agent_node_id) {
                let analyzed_edge = Edge {
                    id:                       EdgeId::new(),
                    from:                     *code_id,
                    to:                       agent_id_parsed,
                    token:                    1,
                    relationship_probability: 1.0,
                    label:                    "analyzed_by".to_string(),
                    metadata:                 HashMap::new(),
                };
                let _ = g.insert_edge(analyzed_edge);
            }
        }
        count
    };

    // Persist.
    {
        let g = graph.lock().unwrap();
        if let Err(e) = persist_fn(&g) {
            eprintln!("[security_agent] persist failed: {e}");
        }
    }

    let violates_count  = violations.iter().filter(|v| v.edge_kind == "violates").count();
    let implements_count = violations.iter().filter(|v| v.edge_kind == "implements").count();
    let related_count   = violations.iter().filter(|v| v.edge_kind == "related_to_rule").count();

    progress.update(|p| {
        p.running     = false;
        p.phase       = AgentPhase::Done;
        p.files_done  = files_total;
        p.current_file = None;
    });

    println!(
        "[security_agent] done: {} files, {} rules checked, {} violations, {} edges written",
        files_total, rules_checked, violates_count, edges_written
    );

    SecurityAgentResult {
        agent_id,
        agent_node_id,
        files_analyzed:   files_total,
        rules_checked,
        violations,
        violates_count,
        implements_count,
        related_count,
        edges_written,
    }
}

// ── Graph helpers ─────────────────────────────────────────────────────────────

/// Create an Agent node in the graph — the audit trail anchor.
fn create_agent_node(
    graph:    &mut DomainGraph,
    agent_id: &str,
    config:   &SecurityAgentConfig,
) -> String {
    let uri  = format!("agent://security/{agent_id}");
    let id   = NodeId::from_content("security_agent", &uri);
    let text = format!(
        "Security Agent run {agent_id}\nScope: {}\nPDF filters: {}",
        config.scope.as_deref().unwrap_or("all"),
        if config.pdf_document_ids.is_empty() {
            "all".to_string()
        } else {
            config.pdf_document_ids.join(", ")
        }
    );

    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(),    "agent".to_string());
    metadata.insert("agent_type".to_string(), "security".to_string());
    metadata.insert("agent_id".to_string(),   agent_id.to_string());
    metadata.insert("source_uri".to_string(), uri.clone());

    let node = Node {
        id,
        domain:      Domain::Custom("agent".to_string()),
        source_uri:  uri,
        source_text: text,
        embeddings:  vec![],
        metadata,
        kind:        NodeKind::Artifcat,
    };

    let _ = graph.insert_node(node);
    id.to_string()
}

// ── Helper trait ──────────────────────────────────────────────────────────────

trait NodeSnapshotExt {
    fn metadata_path_matches(&self, scope: &Option<String>) -> bool;
}

impl NodeSnapshotExt for NodeSnapshot {
    fn metadata_path_matches(&self, scope: &Option<String>) -> bool {
        match scope {
            None => true,
            Some(s) if s.is_empty() => true,
            Some(prefix) => self.path.starts_with(prefix.as_str()),
        }
    }
}

impl NodeId {
    pub fn try_from_str(s: &str) -> Option<Self> {
        uuid::Uuid::parse_str(s).ok().map(NodeId)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_progress_percent() {
        let mut p = AgentProgress::new("test-agent");
        p.files_total = 10;
        p.files_done  = 5;
        assert_eq!(p.percent(), 50);
    }

    #[test]
    fn test_agent_progress_percent_zero_total() {
        let p = AgentProgress::new("test-agent");
        assert_eq!(p.percent(), 0);
    }

    #[test]
    fn test_cosine_sim_identical() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_sim(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_sim_orthogonal() {
        let a = vec![1.0f32, 0.0];
        let b = vec![0.0f32, 1.0];
        assert!(cosine_sim(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_sim_mismatched_dims() {
        let a = vec![1.0f32, 0.0];
        let b = vec![1.0f32];
        assert_eq!(cosine_sim(&a, &b), 0.0);
    }

    #[test]
    fn test_security_agent_config_defaults() {
        let c = SecurityAgentConfig::default();
        assert_eq!(c.similarity_threshold, 0.55);
        assert_eq!(c.top_k_rules, 5);
        assert_eq!(c.max_files, 100);
        assert!(c.pdf_document_ids.is_empty());
        assert!(c.scope.is_none());
    }

    #[test]
    fn test_path_filter() {
        let node = NodeSnapshot {
            id:         NodeId::random(),
            uri:        "codebase://repo/src/auth.rs".to_string(),
            text:       String::new(),
            source:     "codebase".to_string(),
            path:       "src/auth.rs".to_string(),
            symbol:     None,
            filename:   String::new(),
            doc_id:     String::new(),
            embeddings: vec![],
        };

        assert!(node.metadata_path_matches(&None));
        assert!(node.metadata_path_matches(&Some(String::new())));
        assert!(node.metadata_path_matches(&Some("src/".to_string())));
        assert!(!node.metadata_path_matches(&Some("tests/".to_string())));
    }

    #[test]
    fn test_violation_counts() {
        let violations = vec![
            SecurityViolation {
                file_path: "src/a.rs".to_string(),
                symbol: None,
                code_uri: "code://a".to_string(),
                rule_uri: "pdf://r1".to_string(),
                rule_text: "rule".to_string(),
                rule_source: "owasp.pdf".to_string(),
                edge_kind: "violates".to_string(),
                confidence: 0.9,
                explanation: "bad".to_string(),
                severity: "high".to_string(),
            },
            SecurityViolation {
                edge_kind: "implements".to_string(),
                file_path: "src/b.rs".to_string(),
                code_uri: "code://b".to_string(),
                rule_uri: "pdf://r2".to_string(),
                explanation: "good".to_string(),
                severity: "low".to_string(),
                ..Default::default()
            },
            SecurityViolation {
                edge_kind: "related_to_rule".to_string(),
                file_path: "src/c.rs".to_string(),
                code_uri: "code://c".to_string(),
                rule_uri: "pdf://r3".to_string(),
                explanation: "maybe".to_string(),
                severity: "medium".to_string(),
                ..Default::default()
            },
        ];

        let v = violations.iter().filter(|v| v.edge_kind == "violates").count();
        let i = violations.iter().filter(|v| v.edge_kind == "implements").count();
        let r = violations.iter().filter(|v| v.edge_kind == "related_to_rule").count();

        assert_eq!(v, 1);
        assert_eq!(i, 1);
        assert_eq!(r, 1);
    }

    #[test]
    fn test_agent_phase_serialization() {
        let phase = AgentPhase::Analyzing;
        let json  = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"analyzing\"");
    }
}

impl Default for SecurityViolation {
    fn default() -> Self {
        Self {
            file_path:   String::new(),
            symbol:      None,
            code_uri:    String::new(),
            rule_uri:    String::new(),
            rule_text:   String::new(),
            rule_source: String::new(),
            edge_kind:   "related_to_rule".to_string(),
            confidence:  0.5,
            explanation: String::new(),
            severity:    "medium".to_string(),
        }
    }
}