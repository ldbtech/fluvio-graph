//! tool_registry.rs
//!
//! Scans fluvio-tools/src/tools/ at startup, ingests each tool as a
//! NormalizedChunk into the DomainGraph, then uses graph relationships
//! + DomainGraph::similarity_search() for tool detection.
//!
//! Detection algorithm:
//!   Step 1 — Relationship traversal (tags, supports, does_not_support edges)
//!     Strong match (rel_score ≥ 2.0, no blockers) → UseExisting
//!     Weak/no match                                → Step 2
//!
//!   Step 2 — similarity_search() (already in DomainGraph, no duplication)
//!     similarity ≥ 0.82 → Extend existing tool + update node
//!     similarity <  0.82 → Generate new tool   + new node
//!
//! Tool node URI:    tool://architecture/{file_stem}
//! Tag node URI:     tag://tool/{tag}
//! Cap node URI:     cap://tool/{capability}
//! Request node URI: request://tool/{uuid}

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::graph::enums::Domain;
use crate::graph::structs::{DomainGraph, Node};
use crate::ingestion::IngestionPipeline;
use crate::ingestion_registry::connector::{NormalizedChunk, PreDefinedEdge};

// ── Constants ─────────────────────────────────────────────────────────────────

/// similarity_search score threshold — above → Extend, below → Generate.
pub const EXTEND_THRESHOLD:   f32 = 0.82;
/// Minimum relationship traversal score to be a candidate.
pub const MIN_REL_SCORE:      f32 = 1.0;
/// Relationship score at which we skip Step 2 and use the tool directly.
pub const USE_EXISTING_SCORE: f32 = 2.0;

// ── URI helpers ───────────────────────────────────────────────────────────────

pub fn tool_domain() -> Domain {
    Domain::Custom("tools".to_string())
}

pub fn tool_uri(file_stem: &str) -> String {
    format!("tool://architecture/{file_stem}")
}

pub fn tag_uri(tag: &str) -> String {
    format!("tag://tool/{}", tag.trim().to_lowercase().replace(' ', "_"))
}

pub fn cap_uri(cap: &str) -> String {
    format!("cap://tool/{}", cap.trim().to_lowercase().replace(' ', "_"))
}

pub fn request_uri(uuid: &str) -> String {
    format!("request://tool/{uuid}")
}

// ── Header field indices ──────────────────────────────────────────────────────
// Fixed-index array — no HashMap, no heap allocation, stack only.
// Order matches the header format in every .ts tool file.

const TOOL:             usize = 0;
const CATEGORY:         usize = 1;
const DESCRIPTION:      usize = 2;
const STYLES:           usize = 3;
const MATERIALS:        usize = 4;
const SUPPORTS:         usize = 5;
const DOES_NOT_SUPPORT: usize = 6;
const TAGS:             usize = 7;
const VERSION:          usize = 8;
const FIELD_COUNT:      usize = 9;

/// Parse the structured comment header of a .ts tool file into a fixed array.
/// Only scans the first 30 lines — header is always at the top.
/// Returns [Option<&str>; FIELD_COUNT] indexed by the constants above.
fn parse_header(content: &str) -> [Option<&str>; FIELD_COUNT] {
    let mut h: [Option<&str>; FIELD_COUNT] = [None; FIELD_COUNT];

    for line in content.lines().take(30) {
        let line = line.trim();
        if !line.starts_with("//") { continue; }
        let line = line.trim_start_matches('/').trim();
        if line.starts_with('=') { continue; }

        if      let Some(v) = line.strip_prefix("TOOL:")             { h[TOOL]             = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("CATEGORY:")         { h[CATEGORY]         = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("DESCRIPTION:")      { h[DESCRIPTION]      = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("STYLES:")           { h[STYLES]           = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("MATERIALS:")        { h[MATERIALS]        = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("SUPPORTS:")         { h[SUPPORTS]         = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("DOES_NOT_SUPPORT:") { h[DOES_NOT_SUPPORT] = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("TAGS:")             { h[TAGS]             = Some(v.trim()); }
        else if let Some(v) = line.strip_prefix("VERSION:")          { h[VERSION]          = Some(v.trim()); }
    }

    h
}

fn parse_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

// ── ToolMeta ─────────────────────────────────────────────────────────────────

/// Parsed metadata from a tool file's structured header comment block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    pub tool_name:        String,
    pub file_name:        String,
    pub file_path:        PathBuf,
    pub category:         String,
    pub description:      String,
    pub styles:           Vec<String>,
    pub materials:        Vec<String>,
    pub supports:         Vec<String>,
    pub does_not_support: Vec<String>,
    pub tags:             Vec<String>,
    pub version:          String,
    pub is_generated:     bool,
}

impl ToolMeta {
    /// Parse a ToolMeta from the raw content of a .ts file.
    /// Returns None if the file has no structured header.
    pub fn parse(file_path: PathBuf, content: &str) -> Option<Self> {
        // Fail fast — only scan first 30 lines for the marker
        if !content.lines().take(30).any(|l| l.contains("// TOOL:")) {
            return None;
        }

        let h = parse_header(content);

        // TOOL is required — if missing the header is invalid
        let tool_name = h[TOOL]?.to_string();

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let is_generated = file_path
            .to_str()
            .map(|p| p.contains("generated"))
            .unwrap_or(false);

        Some(ToolMeta {
            tool_name,
            file_name,
            file_path,
            category:         h[CATEGORY].unwrap_or("").to_string(),
            description:      h[DESCRIPTION].unwrap_or("").to_string(),
            styles:           h[STYLES].map(parse_csv).unwrap_or_default(),
            materials:        h[MATERIALS].map(parse_csv).unwrap_or_default(),
            supports:         h[SUPPORTS].map(parse_csv).unwrap_or_default(),
            does_not_support: h[DOES_NOT_SUPPORT].map(parse_csv).unwrap_or_default(),
            tags:             h[TAGS].map(parse_csv).unwrap_or_default(),
            version:          h[VERSION].unwrap_or("1.0").to_string(),
            is_generated,
        })
    }

    /// File stem — "sofa" from "sofa.ts"
    pub fn file_stem(&self) -> &str {
        self.file_name.trim_end_matches(".ts")
    }

    /// Source URI for this tool's graph node.
    pub fn uri(&self) -> String {
        tool_uri(self.file_stem())
    }

    /// Embeddable text for semantic search.
    pub fn embeddable_text(&self) -> String {
        format!(
            "tool: {name}\ncategory: {cat}\ndescription: {desc}\n\
             tags: {tags}\nsupports: {supports}\nstyles: {styles}",
            name     = self.tool_name,
            cat      = self.category,
            desc     = self.description,
            tags     = self.tags.join(", "),
            supports = self.supports.join(", "),
            styles   = self.styles.join(", "),
        )
    }

    /// Convert to NormalizedChunk for ingestion into the DomainGraph.
    /// Edges encode the relationships used by the detection algorithm.
    pub fn to_chunk(&self, chunk_index: usize) -> NormalizedChunk {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(),       "tools".to_string());
        metadata.insert("kind".to_string(),         "tool".to_string());
        metadata.insert("file_name".to_string(),    self.file_name.clone());
        metadata.insert("file_path".to_string(),    self.file_path.to_string_lossy().to_string());
        metadata.insert("tool_name".to_string(),    self.tool_name.clone());
        metadata.insert("category".to_string(),     self.category.clone());
        metadata.insert("description".to_string(),  self.description.clone());
        metadata.insert("version".to_string(),      self.version.clone());
        metadata.insert("is_generated".to_string(), self.is_generated.to_string());
        metadata.insert("usage_count".to_string(),  "0".to_string());
        metadata.insert("styles".to_string(),       self.styles.join(","));
        metadata.insert("materials".to_string(),    self.materials.join(","));

        let mut edges: Vec<PreDefinedEdge> = Vec::new();

        // Tag edges — Step 1 keyword matching (strongest signal)
        for tag in &self.tags {
            edges.push(PreDefinedEdge {
                to_uri:                   tag_uri(tag),
                label:                    "tagged".to_string(),
                relationship_probability: 1.0,
                token_cost:               0,
            });
        }

        // Supports edges — positive signal
        for cap in &self.supports {
            edges.push(PreDefinedEdge {
                to_uri:                   cap_uri(cap),
                label:                    "supports".to_string(),
                relationship_probability: 0.95,
                token_cost:               1,
            });
        }

        // Does not support edges — penalty signal
        for cap in &self.does_not_support {
            edges.push(PreDefinedEdge {
                to_uri:                   cap_uri(cap),
                label:                    "does_not_support".to_string(),
                relationship_probability: 0.95,
                token_cost:               1,
            });
        }

        // Category edge
        edges.push(PreDefinedEdge {
            to_uri:                   format!("category://tool/{}", self.category),
            label:                    "in_category".to_string(),
            relationship_probability: 1.0,
            token_cost:               0,
        });

        NormalizedChunk {
            text:              self.embeddable_text(),
            metadata,
            chunk_index,
            source_uri:        self.uri(),
            domain:            tool_domain(),
            pre_defined_edges: edges,
        }
    }
}

// ── DetectResult ──────────────────────────────────────────────────────────────

/// Result of the two-step tool detection algorithm.
#[derive(Debug, Clone)]
pub enum DetectResult {
    /// Strong relationship match, no blockers — use existing tool as-is.
    UseExisting {
        meta:      ToolMeta,
        rel_score: f32,
    },
    /// Partial relationship match + high similarity_search score — extend.
    /// The .ts file gets a new variant. The graph node gets updated edges.
    Extend {
        meta:       ToolMeta,
        rel_score:  f32,
        similarity: f32,
    },
    /// No match or low similarity — generate a brand new tool + new graph node.
    Generate {
        closest_meta: Option<ToolMeta>,
        similarity:   f32,
    },
}

impl DetectResult {
    pub fn action_label(&self) -> &'static str {
        match self {
            DetectResult::UseExisting { .. } => "use_existing",
            DetectResult::Extend { .. }      => "extend",
            DetectResult::Generate { .. }    => "generate",
        }
    }
}

// ── ToolRegistry ──────────────────────────────────────────────────────────────

/// Graph-aware tool registry.
///
/// Startup:   new() → sync_to_graph() — tool nodes live in DomainGraph.
/// Detection: detect() — relationship traversal → similarity_search().
///
/// The graph IS the registry. File paths stored as node metadata.
/// Cosine similarity delegated entirely to DomainGraph::similarity_search().
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    pub tools_dir: PathBuf,
    pub gen_dir:   PathBuf,
}

impl ToolRegistry {
    pub fn new(tools_dir: &str) -> anyhow::Result<Self> {
        let tools_dir = PathBuf::from(tools_dir);
        let gen_dir   = tools_dir.join("generated");
        if !gen_dir.exists() {
            fs::create_dir_all(&gen_dir)?;
        }
        Ok(Self { tools_dir, gen_dir })
    }

    // ── Startup sync ──────────────────────────────────────────────────────────

    /// Scan all .ts tool files and ingest them as nodes into the DomainGraph.
    /// Called once at server startup. Idempotent — skips already-existing nodes.
    pub fn sync_to_graph(&self, pipeline: &mut IngestionPipeline) -> anyhow::Result<usize> {
        let mut count     = 0;
        let mut chunk_idx = 0;

        count += self.ingest_dir(&self.tools_dir, false, &mut chunk_idx, pipeline)?;

        if self.gen_dir.exists() {
            count += self.ingest_dir(&self.gen_dir, true, &mut chunk_idx, pipeline)?;
        }

        tracing::info!("[ToolRegistry] Synced {count} tool nodes to graph");
        Ok(count)
    }

    fn ingest_dir(
        &self,
        dir:          &Path,
        is_generated: bool,
        chunk_idx:    &mut usize,
        pipeline:     &mut IngestionPipeline,
    ) -> anyhow::Result<usize> {
        if !dir.exists() { return Ok(0); }

        const SKIP: &[&str] = &["index.ts", "MaterialLibrary.ts", "interior.ts"];

        let mut count = 0;

        for entry in fs::read_dir(dir)? {
            let entry     = entry?;
            let path      = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if path.extension().and_then(|e| e.to_str()) != Some("ts") { continue; }
            if SKIP.contains(&file_name) { continue; }

            let content = match fs::read_to_string(&path) {
                Ok(c)  => c,
                Err(e) => { tracing::warn!("Cannot read {}: {e}", path.display()); continue; }
            };

            let mut meta = match ToolMeta::parse(path.clone(), &content) {
                Some(m) => m,
                None    => { tracing::warn!("No header in {} — skip", path.display()); continue; }
            };

            meta.is_generated = is_generated;

            // Skip if node already exists in graph
            let uri = meta.uri();
            if pipeline.graph.nodes.values().any(|n| n.source_uri == uri) {
                tracing::debug!("[ToolRegistry] Already in graph: {uri}");
                continue;
            }

            let chunk = meta.to_chunk(*chunk_idx);
            *chunk_idx += 1;

            match pipeline.ingest_normalized_chunks(&[chunk]) {
                Ok((nodes, edges)) => {
                    tracing::info!("[ToolRegistry] {} → {nodes} nodes, {edges} edges", meta.file_name);
                    count += 1;
                }
                Err(e) => tracing::warn!("Failed to ingest {}: {e}", meta.file_name),
            }
        }

        Ok(count)
    }

    // ── Detection ─────────────────────────────────────────────────────────────

    /// Two-step tool detection.
    ///
    /// Step 1: Relationship traversal — score tool nodes by graph edges.
    /// Step 2: DomainGraph::similarity_search() — no duplicate cosine logic.
    pub fn detect(&self, request: &str, pipeline: &IngestionPipeline) -> DetectResult {
        let graph    = &pipeline.graph;
        let keywords = extract_keywords(request);

        // ── Step 1: Relationship traversal ────────────────────────────────────
        let mut candidates: Vec<(&Node, f32, bool)> = graph.nodes.values()
            .filter(|n| Self::is_approved_tool(n))
            .map(|node| {
                let (score, blockers) = Self::rel_score(node, &keywords, graph);
                (node, score, blockers)
            })
            .filter(|(_, score, _)| *score >= MIN_REL_SCORE)
            .collect();

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        if let Some(&(best_node, rel_score, has_blockers)) = candidates.first() {
            let meta = Self::node_to_meta(best_node);

            // Strong match, no blockers → done
            if rel_score >= USE_EXISTING_SCORE && !has_blockers {
                tracing::info!("[ToolRegistry] UseExisting: {} (rel={rel_score:.2})", best_node.source_uri);
                return DetectResult::UseExisting { meta, rel_score };
            }

            // ── Step 2: similarity_search (DomainGraph — no duplication) ──────
            let req_emb = match Self::embed(request, pipeline) {
                Some(e) => e,
                None    => return DetectResult::Generate { closest_meta: Some(meta), similarity: 0.0 },
            };

            // similarity_search scores all nodes — find our best_node's score
            let similarity = graph
                .similarity_search(&req_emb, graph.nodes.len())
                .into_iter()
                .find(|(id, _)| *id == best_node.id)
                .map(|(_, s)| s)
                .unwrap_or(0.0);

            tracing::info!(
                "[ToolRegistry] Step2 {} rel={rel_score:.2} sim={similarity:.3}",
                best_node.source_uri
            );

            return if similarity >= EXTEND_THRESHOLD {
                DetectResult::Extend { meta, rel_score, similarity }
            } else {
                DetectResult::Generate { closest_meta: Some(meta), similarity }
            };
        }

        // ── No relationship match — pure similarity_search ─────────────────
        let req_emb = match Self::embed(request, pipeline) {
            Some(e) => e,
            None    => return DetectResult::Generate { closest_meta: None, similarity: 0.0 },
        };

        // Search all nodes, filter to approved tool nodes only
        let best = graph
            .similarity_search(&req_emb, graph.nodes.len())
            .into_iter()
            .filter_map(|(id, score)| {
                graph.nodes.get(&id)
                    .filter(|n| Self::is_approved_tool(n))
                    .map(|n| (n, score))
            })
            .next();

        match best {
            Some((node, similarity)) => {
                tracing::info!(
                    "[ToolRegistry] No rel match. Closest: {} sim={similarity:.3}",
                    node.source_uri
                );
                let meta = Self::node_to_meta(node);
                if similarity >= EXTEND_THRESHOLD {
                    DetectResult::Extend { meta, rel_score: 0.0, similarity }
                } else {
                    DetectResult::Generate { closest_meta: Some(meta), similarity }
                }
            }
            None => {
                tracing::info!("[ToolRegistry] No match at all — Generate");
                DetectResult::Generate { closest_meta: None, similarity: 0.0 }
            }
        }
    }

    // ── Graph utilities ───────────────────────────────────────────────────────

    /// Score a tool node against extracted keywords using its graph edges.
    /// Returns (score, has_blockers).
    fn rel_score(node: &Node, keywords: &[String], graph: &DomainGraph) -> (f32, bool) {
        let mut score        = 0.0_f32;
        let mut has_blockers = false;

        let edges = match graph.adj.get(&node.id) {
            Some(e) => e,
            None    => return (0.0, false),
        };

        for edge in edges {
            let target_uri = graph.nodes
                .get(&edge.to)
                .map(|n| n.source_uri.as_str())
                .unwrap_or("");

            let kw_match = keywords.iter()
                .any(|kw| target_uri.contains(kw.as_str()));

            match edge.label.as_str() {
                "tagged"           if kw_match => { score += 3.0; }
                "supports"         if kw_match => { score += 1.5; }
                "in_category"      if kw_match => { score += 1.0; }
                "does_not_support" if kw_match => {
                    score        -= 2.5;
                    has_blockers  = true;
                }
                _ => {}
            }
        }

        // Description word bonus
        let desc = node.metadata.get("description").map(|d| d.as_str()).unwrap_or("");
        for kw in keywords {
            if desc.to_lowercase().contains(kw.as_str()) {
                score += 0.5;
            }
        }

        (score.max(0.0), has_blockers)
    }

    /// True if the node is an approved (non-generated) tool.
    fn is_approved_tool(n: &Node) -> bool {
        n.metadata.get("kind").map(|k| k == "tool").unwrap_or(false)
            && !n.metadata.get("is_generated").map(|v| v == "true").unwrap_or(false)
    }

    /// Embed request text using the pipeline's EmbeddingContext.
    fn embed(request: &str, pipeline: &IngestionPipeline) -> Option<Vec<f32>> {
        pipeline.embed_ctx.lock().ok()
            .and_then(|mut ctx| ctx.embed(request).ok())
            .filter(|e| !e.is_empty())
    }

    /// Reconstruct ToolMeta from a graph node's metadata.
    pub fn node_to_meta(node: &Node) -> ToolMeta {
        let get = |k: &str| node.metadata.get(k).cloned().unwrap_or_default();
        let csv = |k: &str| -> Vec<String> {
            let v = get(k);
            if v.is_empty() { vec![] }
            else { v.split(',').map(|s| s.trim().to_string()).collect() }
        };

        ToolMeta {
            tool_name:        get("tool_name"),
            file_name:        get("file_name"),
            file_path:        PathBuf::from(get("file_path")),
            category:         get("category"),
            description:      get("description"),
            styles:           csv("styles"),
            materials:        csv("materials"),
            supports:         vec![],  // live in graph edges, not metadata
            does_not_support: vec![],  // live in graph edges, not metadata
            tags:             vec![],  // live in graph edges, not metadata
            version:          get("version"),
            is_generated:     get("is_generated") == "true",
        }
    }

    /// File path from a graph node's metadata.
    pub fn file_path_from_node(node: &Node) -> Option<PathBuf> {
        node.metadata.get("file_path").map(PathBuf::from)
    }

    /// Promote a generated tool: move .ts file + update graph node metadata.
    pub fn promote(&self, file_name: &str, graph: &mut DomainGraph) -> anyhow::Result<PathBuf> {
        let from = self.gen_dir.join(file_name);
        let to   = self.tools_dir.join(file_name);

        if !from.exists() {
            anyhow::bail!("generated tool not found: {}", from.display());
        }

        fs::copy(&from, &to)?;
        fs::remove_file(&from)?;

        let uri = tool_uri(file_name.trim_end_matches(".ts"));
        for node in graph.nodes.values_mut() {
            if node.source_uri == uri {
                node.metadata.insert("is_generated".to_string(), "false".to_string());
                node.metadata.insert("file_path".to_string(), to.to_string_lossy().to_string());
                break;
            }
        }

        tracing::info!("[ToolRegistry] Promoted: {file_name}");
        Ok(to)
    }

    /// Increment usage_count on a tool node when it is used.
    pub fn record_usage(file_stem: &str, graph: &mut DomainGraph) {
        let uri = tool_uri(file_stem);
        for node in graph.nodes.values_mut() {
            if node.source_uri == uri {
                let count: u64 = node.metadata
                    .get("usage_count")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                node.metadata.insert("usage_count".to_string(), (count + 1).to_string());
                break;
            }
        }
    }

    /// Path for a new generated tool file.
    pub fn generated_path(&self, file_name: &str) -> PathBuf {
        self.gen_dir.join(file_name)
    }
}

// ── extract_keywords ──────────────────────────────────────────────────────────

/// Extract meaningful keywords — strips stopwords and short words.
pub fn extract_keywords(request: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "in", "on", "at",
        "to", "for", "of", "with", "my", "i", "need", "want",
        "please", "can", "you", "me", "add", "put", "place",
        "make", "create", "generate", "build", "give", "get",
        "some", "have", "has", "be", "is", "are", "was",
    ];

    request
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.trim().to_string())
        .filter(|w| w.len() > 2 && !STOPWORDS.contains(&w.as_str()))
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SOFA_CONTENT: &str = r#"
// ============================================================
// TOOL: Standard Sofa
// FILE: sofa.ts
// CATEGORY: seating
// DESCRIPTION: Three-seat sofa with cushioned back and arms.
// STYLES: modern, scandinavian, industrial
// MATERIALS: fabric_grey, fabric_cream, white_oak
// SUPPORTS: arm styles, cushion count, width scaling
// DOES_NOT_SUPPORT: sectional, chaise lounge, L-shape
// TAGS: sofa, couch, seating, living room
// VERSION: 1.0
// ============================================================
"#;

    const BED_CONTENT: &str = r#"
// ============================================================
// TOOL: Standard Bed
// FILE: bed.ts
// CATEGORY: bedroom
// DESCRIPTION: Bed frame with mattress and headboard.
// STYLES: modern, platform, upholstered
// MATERIALS: white_oak, dark_walnut, fabric_grey
// SUPPORTS: size variants, headboard height
// DOES_NOT_SUPPORT: bunk bed, canopy, murphy bed
// TAGS: bed, bedroom, sleeping, frame, mattress
// VERSION: 1.0
// ============================================================
"#;

    #[test]
    fn test_parse_header_array() {
        let h = parse_header(SOFA_CONTENT);
        assert_eq!(h[TOOL],        Some("Standard Sofa"));
        assert_eq!(h[CATEGORY],    Some("seating"));
        assert_eq!(h[DESCRIPTION], Some("Three-seat sofa with cushioned back and arms."));
        assert_eq!(h[VERSION],     Some("1.0"));
        assert!(h[TAGS].is_some());
    }

    #[test]
    fn test_parse_tool_meta() {
        let meta = ToolMeta::parse(PathBuf::from("sofa.ts"), SOFA_CONTENT).unwrap();
        assert_eq!(meta.tool_name,  "Standard Sofa");
        assert_eq!(meta.category,   "seating");
        assert_eq!(meta.file_stem(), "sofa");
        assert_eq!(meta.uri(),      "tool://architecture/sofa");
        assert!(meta.tags.contains(&"sofa".to_string()));
        assert!(meta.does_not_support.contains(&"sectional".to_string()));
        assert!(!meta.is_generated);
    }

    #[test]
    fn test_parse_generated_flag() {
        let path = PathBuf::from("generated/spiral_staircase.ts");
        let meta = ToolMeta::parse(path, SOFA_CONTENT).unwrap();
        assert!(meta.is_generated);
    }

    #[test]
    fn test_parse_no_header_returns_none() {
        let content = "export function foo() {}";
        assert!(ToolMeta::parse(PathBuf::from("x.ts"), content).is_none());
    }

    #[test]
    fn test_parse_missing_tool_field_returns_none() {
        let content = "// CATEGORY: seating\n// TAGS: sofa\n";
        assert!(ToolMeta::parse(PathBuf::from("x.ts"), content).is_none());
    }

    #[test]
    fn test_uri_formats() {
        assert_eq!(tool_uri("sofa"),       "tool://architecture/sofa");
        assert_eq!(tag_uri("couch"),       "tag://tool/couch");
        assert_eq!(tag_uri("living room"), "tag://tool/living_room");
        assert_eq!(cap_uri("arm styles"),  "cap://tool/arm_styles");
        assert_eq!(request_uri("abc"),     "request://tool/abc");
    }

    #[test]
    fn test_to_chunk_edges() {
        let meta  = ToolMeta::parse(PathBuf::from("sofa.ts"), SOFA_CONTENT).unwrap();
        let chunk = meta.to_chunk(0);

        let tagged: Vec<_> = chunk.pre_defined_edges.iter()
            .filter(|e| e.label == "tagged").collect();
        assert_eq!(tagged.len(), meta.tags.len());

        let blocked: Vec<_> = chunk.pre_defined_edges.iter()
            .filter(|e| e.label == "does_not_support").collect();
        assert!(!blocked.is_empty());

        let supported: Vec<_> = chunk.pre_defined_edges.iter()
            .filter(|e| e.label == "supports").collect();
        assert!(!supported.is_empty());

        assert_eq!(chunk.source_uri, "tool://architecture/sofa");
        assert_eq!(chunk.domain,     tool_domain());
    }

    #[test]
    fn test_to_chunk_metadata() {
        let meta  = ToolMeta::parse(PathBuf::from("sofa.ts"), SOFA_CONTENT).unwrap();
        let chunk = meta.to_chunk(0);
        assert_eq!(chunk.metadata["kind"],        "tool");
        assert_eq!(chunk.metadata["category"],    "seating");
        assert_eq!(chunk.metadata["usage_count"], "0");
        assert_eq!(chunk.metadata["version"],     "1.0");
    }

    #[test]
    fn test_embeddable_text() {
        let meta = ToolMeta::parse(PathBuf::from("sofa.ts"), SOFA_CONTENT).unwrap();
        let text = meta.embeddable_text();
        assert!(text.contains("Standard Sofa"));
        assert!(text.contains("seating"));
        assert!(text.contains("sofa"));
    }

    #[test]
    fn test_detect_result_labels() {
        let meta = ToolMeta::parse(PathBuf::from("sofa.ts"), SOFA_CONTENT).unwrap();

        let r1 = DetectResult::UseExisting { meta: meta.clone(), rel_score: 3.0 };
        assert_eq!(r1.action_label(), "use_existing");

        let r2 = DetectResult::Extend { meta: meta.clone(), rel_score: 1.0, similarity: 0.85 };
        assert_eq!(r2.action_label(), "extend");

        let r3 = DetectResult::Generate { closest_meta: None, similarity: 0.3 };
        assert_eq!(r3.action_label(), "generate");
    }

    #[test]
    fn test_extract_keywords() {
        let kws = extract_keywords("I need a curved sectional sofa with chaise lounge");
        assert!(kws.contains(&"curved".to_string()));
        assert!(kws.contains(&"sectional".to_string()));
        assert!(kws.contains(&"sofa".to_string()));
        assert!(kws.contains(&"chaise".to_string()));
        assert!(!kws.contains(&"need".to_string()));
        assert!(!kws.contains(&"with".to_string()));
        assert!(!kws.contains(&"a".to_string()));
    }

    #[test]
    fn test_thresholds_sanity() {
        assert!(EXTEND_THRESHOLD   > 0.0 && EXTEND_THRESHOLD < 1.0);
        assert!(MIN_REL_SCORE      > 0.0);
        assert!(USE_EXISTING_SCORE > MIN_REL_SCORE);
    }

    #[test]
    fn test_parse_csv() {
        let result = parse_csv("sofa, couch, seating, living room");
        assert_eq!(result, vec!["sofa", "couch", "seating", "living room"]);
    }

    #[test]
    fn test_header_only_scans_30_lines() {
        // TOOL: marker buried after line 30 — should not be found
        let mut content = String::new();
        for _ in 0..31 {
            content.push_str("// some other comment\n");
        }
        content.push_str("// TOOL: Hidden Tool\n");
        assert!(ToolMeta::parse(PathBuf::from("x.ts"), &content).is_none());
    }
}