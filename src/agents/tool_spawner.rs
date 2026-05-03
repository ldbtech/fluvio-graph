//! tool_spawner.rs
//!
//! Two-agent tool generation pipeline:
//!
//!   Agent 1 — SpecWriter:
//!     Reads user request → writes a detailed .md spec file
//!     The spec is the single source of truth for what to build
//!
//!   Agent 2 — CodeGenerator:
//!     Reads the .md spec → writes the tool file (.ts / future languages)
//!     No prompt engineering — the spec IS the instruction
//!
//! Flow:
//!   spawn(request, domain)
//!     → detect()              — UseExisting / Extend / Generate
//!     → write_spec()          — Agent 1 writes .md
//!     → generate_from_spec()  — Agent 2 reads .md, writes tool file
//!     → ingest into graph
//!
//! Generic across domains — register any ToolDomain, zero changes to spawner.
//! Generic across languages — implement CodeGenerator for any language.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agents::tool_registry::{
    DetectResult, ToolMeta, ToolRegistry,
    extract_keywords, tool_uri,
};
use crate::graph::fluvio_graph::FluvioGraph;
use crate::graph::structs::{DomainGraph, Edge, EdgeId, NodeId};
use crate::ingestion::IngestionPipeline;

use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

// ── CodeGenerator trait ───────────────────────────────────────────────────────

/// Translates a .md spec file into source code.
/// Implement this trait to support new languages.
///
/// The spec file IS the instruction — no additional prompting needed.
/// Agent 2 just reads the spec and builds exactly what it describes.
#[async_trait::async_trait]
pub trait CodeGenerator: Send + Sync {
    /// File extension this generator produces — "ts", "py", "rs"
    fn extension(&self) -> &str;

    /// Read the spec file and generate source code.
    ///
    /// `spec_path`  — path to the .md spec written by Agent 1
    /// `base_code`  — existing file content when extending, None when generating new
    /// `api_key`    — Claude API key
    async fn generate_from_spec(
        &self,
        spec_path: &PathBuf,
        base_code: Option<&str>,
        api_key:   &str,
    ) -> anyhow::Result<String>;
}

// ── TypeScriptGenerator ───────────────────────────────────────────────────────

/// Generates Three.js TypeScript tool files from a .md spec.
pub struct TypeScriptGenerator;

#[async_trait::async_trait]
impl CodeGenerator for TypeScriptGenerator {
    fn extension(&self) -> &str { "ts" }

    async fn generate_from_spec(
        &self,
        spec_path: &PathBuf,
        base_code: Option<&str>,
        api_key:   &str,
    ) -> anyhow::Result<String> {
        let spec = fs::read_to_string(spec_path)
            .map_err(|e| anyhow::anyhow!("cannot read spec {}: {e}", spec_path.display()))?;

        let prompt = match base_code {
            // ── Generate new tool ─────────────────────────────────────────────
            None => format!(
                r#"You are a TypeScript developer building Three.js architectural tools.

Read this spec and implement the tool exactly as described:

{spec}

Implementation rules:
1. The file MUST start with this header block (fill values from spec):
// ============================================================
// TOOL: <tool name from spec>
// FILE: <file_stem>.ts
// CATEGORY: <category from spec>
// DESCRIPTION: <description from spec>
// STYLES: <styles from spec>
// MATERIALS: fabric_grey, fabric_cream, white_oak, dark_walnut, marble, brushed_steel
// SUPPORTS: <from spec>
// DOES_NOT_SUPPORT: <known limitations>
// TAGS: <tags from spec>
// VERSION: 1.0
// ============================================================

2. Imports:
import * as THREE from "three"
import {{ MaterialLibrary, MaterialKey }} from "./MaterialLibrary"

3. Export one function:
export function generate<PascalName>(style: string, material: MaterialKey): THREE.Group

4. Geometry rules:
   - Use ONLY: THREE.BoxGeometry, THREE.CylinderGeometry, THREE.SphereGeometry, THREE.CapsuleGeometry
   - All dimensions in meters (real world scale — a sofa is ~2.2m wide)
   - All meshes: castShadow = true, receiveShadow = true
   - Name every mesh component: mesh.name = "component_name"
   - group.userData = {{ tool: "file_stem", style, material }}

5. Material:
   const mat = new THREE.MeshStandardMaterial({{
     color: MaterialLibrary[material]?.color ?? '#888888',
     roughness: MaterialLibrary[material]?.roughness ?? 0.8,
     metalness: MaterialLibrary[material]?.metalness ?? 0.0,
   }})

Return ONLY the TypeScript file — no explanation, no markdown backticks."#
            ),

            // ── Extend existing tool ──────────────────────────────────────────
            Some(existing) => format!(
                r#"You are extending an existing Three.js TypeScript tool.

EXISTING FILE (keep ALL of this — only add new code):
{existing}

EXTENSION SPEC (what to add):
{spec}

Extension rules:
1. Keep every line of the existing file intact
2. Add a new exported function for the new variant, OR extend the existing
   function with a new style case — whichever fits better
3. Update the header comment block:
   - Add new capabilities to SUPPORTS
   - Remove from DOES_NOT_SUPPORT if listed there
   - Bump VERSION by 0.1 (e.g. 1.0 → 1.1)
4. Use ONLY: BoxGeometry, CylinderGeometry, SphereGeometry, CapsuleGeometry
5. All new meshes: castShadow = true, receiveShadow = true

Return ONLY the complete updated TypeScript file — no explanation, no markdown."#
            ),
        };

        call_claude(&prompt, api_key).await
    }
}

// ── ToolSpec ──────────────────────────────────────────────────────────────────

/// Minimal parsed fields from a .md spec file.
/// Used only for graph node creation — the full text goes to CodeGenerator.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub tool_name:  String,
    pub file_stem:  String,
    pub category:   String,
    pub base_tool:  Option<String>,
    pub tags:       Vec<String>,
    pub styles:     Vec<String>,
    pub description: String,
}

impl ToolSpec {
    pub fn parse(content: &str) -> anyhow::Result<Self> {
        let get = |key: &str| -> String {
            content.lines()
                .find(|l| {
                    let trimmed = l.trim().trim_start_matches("//").trim();
                    trimmed.starts_with(key)
                })
                .map(|l| {
                    let trimmed = l.trim().trim_start_matches("//").trim();
                    trimmed.strip_prefix(key)
                          .unwrap_or("")
                          .trim()
                          .trim_start_matches(':')
                          .trim()
                          .to_string()
                })
                .unwrap_or_default()
        };

        let tool_name = get("TOOL_NAME");
        let file_stem = get("FILE_STEM");

        if tool_name.is_empty() || file_stem.is_empty() {
            anyhow::bail!("spec missing TOOL_NAME or FILE_STEM");
        }

        let base = get("BASE_TOOL");
        let base_tool = if base.is_empty() { None } else { Some(base) };

        let tags   = csv(&extract_md_section(content, "Tags"));
        let styles = csv(&extract_md_section(content, "Styles"));
        let description = extract_md_section(content, "Description");

        Ok(ToolSpec { tool_name, file_stem, category: get("CATEGORY"), base_tool, tags, styles, description })
    }
}

fn csv(s: &str) -> Vec<String> {
    s.split(',').map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).collect()
}

fn extract_md_section(content: &str, name: &str) -> String {
    let header = format!("## {name}");
    let mut in_section = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim() == header { in_section = true; continue; }
        if in_section {
            if line.starts_with("## ") { break; }
            lines.push(line);
        }
    }
    lines.join("\n").trim().to_string()
}

// ── JobManifest ───────────────────────────────────────────────────────────────

/// Snapshot of a file before modification.
#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub path:             PathBuf,
    pub previous_content: String,
}

/// Everything a job created or modified.
/// Call rollback() to undo on cancellation.
#[derive(Debug, Clone, Default)]
pub struct JobManifest {
    pub job_id:         String,
    pub created_files:  Vec<PathBuf>,
    pub modified_files: Vec<FileSnapshot>,
    pub created_nodes:  Vec<String>,
}

impl JobManifest {
    pub fn new(job_id: &str) -> Self {
        Self { job_id: job_id.to_string(), ..Default::default() }
    }

    pub fn track_created(&mut self, path: PathBuf) {
        self.created_files.push(path);
    }

    pub fn track_modified(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        let previous_content = fs::read_to_string(path)?;
        self.modified_files.push(FileSnapshot { path: path.clone(), previous_content });
        Ok(())
    }

    pub fn track_node(&mut self, uri: String) {
        self.created_nodes.push(uri);
    }

    /// Undo everything — delete created files, restore modified files, remove graph nodes.
    pub fn rollback(&self, graph: &mut DomainGraph) {
        for path in &self.created_files {
            if path.exists() {
                let _ = fs::remove_file(path);
                tracing::info!("[Rollback] Deleted: {}", path.display());
            }
        }
        for snap in &self.modified_files {
            let _ = fs::write(&snap.path, &snap.previous_content);
            tracing::info!("[Rollback] Restored: {}", snap.path.display());
        }
        for uri in &self.created_nodes {
            let ids: Vec<NodeId> = graph.nodes.values()
                .filter(|n| &n.source_uri == uri)
                .map(|n| n.id)
                .collect();
            for id in ids {
                graph.delete_node(id);
                tracing::info!("[Rollback] Removed node: {uri}");
            }
        }
    }
}

// ── SpawnResult ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResult {
    pub action:    String,           // "use_existing" | "extended" | "generated"
    pub file_name: String,
    pub file_path: PathBuf,
    pub spec_path: Option<PathBuf>,  // None if UseExisting
    pub tool_name: String,
    pub is_new:    bool,
    pub job_id:    Option<String>,   // None if UseExisting
}

// ── ToolDomain ────────────────────────────────────────────────────────────────

/// A domain is a set of tools + specs + a code generator.
/// Adding a new domain = instantiate a ToolDomain, zero spawner changes.
pub struct ToolDomain {
    pub name:      String,
    pub tools_dir: PathBuf,
    pub specs_dir: PathBuf,
    pub registry:  ToolRegistry,
    pub generator: Box<dyn CodeGenerator>,
}

impl ToolDomain {
    /// Create a TypeScript domain — the most common case.
    pub fn typescript(name: &str, tools_dir: &str, specs_dir: &str) -> anyhow::Result<Self> {
        fs::create_dir_all(specs_dir)?;
        Ok(Self {
            name:      name.to_string(),
            tools_dir: PathBuf::from(tools_dir),
            specs_dir: PathBuf::from(specs_dir),
            registry:  ToolRegistry::new(tools_dir)?,
            generator: Box::new(TypeScriptGenerator),
        })
    }

    pub fn spec_path(&self, file_stem: &str) -> PathBuf {
        self.specs_dir.join(format!("{file_stem}.md"))
    }

    pub fn generated_path(&self, file_stem: &str) -> PathBuf {
        self.registry.generated_path(
            &format!("{file_stem}.{}", self.generator.extension())
        )
    }
}

// ── ToolSpawner ───────────────────────────────────────────────────────────────

/// Generic domain-agnostic tool spawner.
///
/// Registers multiple ToolDomains.
/// Routes spawn() calls to the right domain.
/// Internally chains Agent 1 (spec writer) + Agent 2 (code generator).
pub struct ToolSpawner {
    pub domains: HashMap<String, ToolDomain>,
    pub api_key: String,
}

impl ToolSpawner {
    pub fn new(api_key: String) -> Self {
        Self { domains: HashMap::new(), api_key }
    }

    /// Register a domain. Call once per domain at startup.
    pub fn add_domain(&mut self, domain: ToolDomain) {
        tracing::info!("[ToolSpawner] Registered domain: {}", domain.name);
        self.domains.insert(domain.name.clone(), domain);
    }

    /// Sync all registered domains to the graph on startup.
    pub fn sync_all(&self, pipeline: &mut IngestionPipeline) -> anyhow::Result<()> {
        for domain in self.domains.values() {
            let n = domain.registry.sync_to_graph(pipeline)?;
            tracing::info!("[ToolSpawner] {} → {n} tools synced", domain.name);
        }
        Ok(())
    }

    /// Main entry point.
    ///
    /// Internally:
    ///   1. detect()        — figure out action
    ///   2. write_spec()    — Agent 1 writes .md
    ///   3. generate_from_spec() — Agent 2 reads .md, writes tool file
    ///   4. ingest          — graph node created/updated
    pub async fn spawn(
        &self,
        request:     &str,
        domain_name: &str,
        pipeline:    &mut IngestionPipeline,
        manifest:    &mut JobManifest,
    ) -> anyhow::Result<SpawnResult> {
        let domain = self.domains.get(domain_name)
            .ok_or_else(|| anyhow::anyhow!("unknown domain: {domain_name}"))?;

        // ── Step 1: detect ────────────────────────────────────────────────────
        let detect = domain.registry.detect(request, pipeline);
        tracing::info!(
            "[ToolSpawner] domain={domain_name} action={} request='{request}'",
            detect.action_label()
        );

        match detect {
            // ── UseExisting — nothing to build ────────────────────────────────
            DetectResult::UseExisting { meta, rel_score } => {
                tracing::info!(
                    "[ToolSpawner] UseExisting: {} (rel={rel_score:.2})",
                    meta.file_name
                );
                ToolRegistry::record_usage(meta.file_stem(), &mut pipeline.graph);
                Ok(SpawnResult {
                    action:    "use_existing".to_string(),
                    file_name: meta.file_name.clone(),
                    file_path: meta.file_path.clone(),
                    spec_path: None,
                    tool_name: meta.tool_name.clone(),
                    is_new:    false,
                    job_id:    None,
                })
            }

            // ── Extend — Agent 1 writes spec, Agent 2 extends existing file ───
            DetectResult::Extend { meta, rel_score, similarity } => {
                tracing::info!(
                    "[ToolSpawner] Extend: {} (rel={rel_score:.2} sim={similarity:.3})",
                    meta.file_name
                );

                // Agent 1: write spec
                let spec_path = self.write_spec(
                    request, &meta.file_stem().to_string(),
                    domain, Some(&meta), manifest,
                ).await?;

                // Snapshot existing file before modifying
                manifest.track_modified(&meta.file_path)?;
                let existing = fs::read_to_string(&meta.file_path)?;

                // Agent 2: read spec + extend existing file
                let code = domain.generator
                    .generate_from_spec(&spec_path, Some(&existing), &self.api_key)
                    .await?;

                fs::write(&meta.file_path, &code)?;

                // Update graph node
                self.update_node_after_extend(&meta, request, pipeline);

                tracing::info!("[ToolSpawner] Extended: {}", meta.file_name);
                Ok(SpawnResult {
                    action:    "extended".to_string(),
                    file_name: meta.file_name.clone(),
                    file_path: meta.file_path.clone(),
                    spec_path: Some(spec_path),
                    tool_name: meta.tool_name.clone(),
                    is_new:    false,
                    job_id:    Some(manifest.job_id.clone()),
                })
            }

            // ── Generate — Agent 1 writes spec, Agent 2 builds new file ───────
            DetectResult::Generate { closest_meta, similarity } => {
                tracing::info!(
                    "[ToolSpawner] Generate (closest sim={similarity:.3})"
                );

                let file_stem = derive_file_stem(request);
                let file_name = format!("{file_stem}.{}", domain.generator.extension());
                let file_path = domain.generated_path(&file_stem);

                // Agent 1: write spec
                let spec_path = self.write_spec(
                    request, &file_stem,
                    domain, closest_meta.as_ref(), manifest,
                ).await?;

                // Agent 2: read spec, generate new file
                let code = domain.generator
                    .generate_from_spec(&spec_path, None, &self.api_key)
                    .await?;

                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                manifest.track_created(file_path.clone());
                fs::write(&file_path, &code)?;

                // Ingest into graph
                let tool_meta = crate::agents::tool_registry::ToolMeta::parse(
                    file_path.clone(), &code,
                ).unwrap_or_else(|| self.fallback_meta(request, &file_stem, &file_path, domain));

                let uri   = tool_meta.uri();
                let chunk = tool_meta.to_chunk(0);
                manifest.track_node(uri.clone());

                pipeline.ingest_normalized_chunks(&[chunk])
                    .map_err(|e| anyhow::anyhow!("graph ingest failed: {e:?}"))?;

                // Wire extends edge if we have a closest tool
                if let Some(closest) = &closest_meta {
                    self.wire_extends_edge(&uri, &closest.uri(), pipeline);
                }

                tracing::info!("[ToolSpawner] Generated: {file_name}");
                Ok(SpawnResult {
                    action:    "generated".to_string(),
                    tool_name: tool_meta.tool_name,
                    file_name,
                    file_path,
                    spec_path: Some(spec_path),
                    is_new:    true,
                    job_id:    Some(manifest.job_id.clone()),
                })
            }
        }
    }

    // ── Agent 1: Spec writer ──────────────────────────────────────────────────

    /// Agent 1 — reads the user request and writes a detailed .md spec.
    /// The spec becomes the instruction for Agent 2.
    async fn write_spec(
        &self,
        request:   &str,
        file_stem: &str,
        domain:    &ToolDomain,
        base_tool: Option<&ToolMeta>,
        manifest:  &mut JobManifest,
    ) -> anyhow::Result<PathBuf> {
        let base_hint = base_tool.map(|m| format!(
            "\n\nThis extends an existing tool:\n\
             TOOL: {}\nDESCRIPTION: {}\nFILE: {}\n\
             Only describe the NEW capabilities to add.",
            m.tool_name, m.description, m.file_name
        )).unwrap_or_default();

        let ext = domain.generator.extension();

        let prompt = format!(
            r#"You are a tool architect planning a new {domain} tool.

USER REQUEST: "{request}"{base_hint}

Write a structured spec file in this EXACT format
(use // KEY: VALUE for the frontmatter, ## Section for bodies):

// TOOL_NAME: <display name e.g. "Curved Sectional Sofa">
// FILE_STEM: {file_stem}
// CATEGORY: <seating|bedroom|tables|office|structure|lighting|electronics|kitchen|nature|furniture>
// REQUEST: {request}
// BASE_TOOL: <file_stem of base tool if extending, blank if new>
// STATUS: pending
// CREATED_AT: {now}

## Description
<one clear sentence describing what this tool generates>

## Components
<bullet list — every physical part that makes up this object>
<e.g. - Seat cushion: 2.2m × 0.9m × 0.2m, cream fabric>

## Dimensions
<width, depth, height in meters — realistic real world measurements>

## Materials
<primary material key, frame material key, etc>
<use keys from: fabric_grey, fabric_cream, fabric_navy, white_oak, dark_walnut,>
<pine, polished_concrete, raw_concrete, marble, slate, terracotta,>
<brushed_brass, brushed_steel, matte_black, glass>

## Three.js Approach
<exactly how to build this using {ext} geometry primitives>
<specify which geometry type per component>

## Styles
<comma separated — e.g. modern, scandinavian, industrial>

## Tags
<comma separated search keywords>

Return ONLY the spec file — no explanation, no markdown backticks."#,
            domain = domain.name,
            now    = chrono::Utc::now().to_rfc3339(),
        );

        let spec_content = call_claude(&prompt, &self.api_key).await?;
        let spec_path    = domain.spec_path(file_stem);

        if let Some(parent) = spec_path.parent() {
            fs::create_dir_all(parent)?;
        }
        manifest.track_created(spec_path.clone());
        fs::write(&spec_path, &spec_content)?;

        tracing::info!("[ToolSpawner] Spec written: {}", spec_path.display());
        Ok(spec_path)
    }

    // ── Approve ───────────────────────────────────────────────────────────────

    /// Promote a generated tool from generated/ to tools/.
    pub fn approve(
        &self,
        file_name:   &str,
        domain_name: &str,
        pipeline:    &mut IngestionPipeline,
    ) -> anyhow::Result<PathBuf> {
        let domain = self.domains.get(domain_name)
            .ok_or_else(|| anyhow::anyhow!("unknown domain: {domain_name}"))?;
        let path = domain.registry.promote(file_name, &mut pipeline.graph)?;
        tracing::info!("[ToolSpawner] Approved: {file_name}");
        Ok(path)
    }

    // ── Graph helpers ─────────────────────────────────────────────────────────

    fn update_node_after_extend(
        &self,
        meta:     &ToolMeta,
        request:  &str,
        pipeline: &mut IngestionPipeline,
    ) {
        let uri      = meta.uri();
        let addition = extract_keywords(request).join(", ");

        for node in pipeline.graph.nodes.values_mut() {
            if node.source_uri != uri { continue; }
            if let Some(v) = node.metadata.get("version").cloned() {
                node.metadata.insert("version".to_string(), bump_version(&v));
            }
            let desc = node.metadata.get("description").cloned().unwrap_or_default();
            node.metadata.insert("description".to_string(), format!("{desc} + {addition}"));
            break;
        }
    }

    fn wire_extends_edge(
        &self,
        from_uri: &str,
        to_uri:   &str,
        pipeline: &mut IngestionPipeline,
    ) {
        let from_id = pipeline.graph.nodes.values()
            .find(|n| n.source_uri == from_uri).map(|n| n.id);
        let to_id = pipeline.graph.nodes.values()
            .find(|n| n.source_uri == to_uri).map(|n| n.id);

        if let (Some(from), Some(to)) = (from_id, to_id) {
            let _ = pipeline.graph.insert_edge(Edge {
                id:                       EdgeId::new(),
                from, to,
                token:                    0,
                relationship_probability: 0.9,
                label:                    "extends".to_string(),
                metadata:                 HashMap::new(),
            });
            tracing::info!("[ToolSpawner] Wired extends: {from_uri} → {to_uri}");
        }
    }

    fn fallback_meta(
        &self,
        request:   &str,
        file_stem: &str,
        file_path: &PathBuf,
        domain:    &ToolDomain,
    ) -> crate::agents::tool_registry::ToolMeta {
        crate::agents::tool_registry::ToolMeta {
            tool_name:        title_case(file_stem),
            file_name:        format!("{file_stem}.{}", domain.generator.extension()),
            file_path:        file_path.clone(),
            category:         infer_category(request),
            description:      request.to_string(),
            styles:           vec![],
            materials:        vec![],
            supports:         extract_keywords(request),
            does_not_support: vec![],
            tags:             extract_keywords(request),
            version:          "1.0".to_string(),
            is_generated:     true,
        }
    }
}

// ── Claude API ────────────────────────────────────────────────────────────────

async fn call_claude(prompt: &str, api_key: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key",         api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type",      "application/json")
        .json(&serde_json::json!({
            "model":      "claude-sonnet-4-20250514",
            "max_tokens": 3000,
            "messages":   [{ "role": "user", "content": prompt }]
        }))
        .send().await?
        .json::<serde_json::Value>().await?;

    let text = res["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("no text in Claude response: {res}"))?
        .trim()
        .trim_start_matches("```typescript")
        .trim_start_matches("```markdown")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();

    if text.is_empty() {
        anyhow::bail!("Claude returned empty response");
    }
    Ok(text)
}

// ── String helpers ────────────────────────────────────────────────────────────

/// "I need a TV stand" → "tv_stand"
fn derive_file_stem(request: &str) -> String {
    extract_keywords(request).into_iter().take(4).collect::<Vec<_>>().join("_")
}

/// "tv_stand" → "Tv Stand"
pub fn title_case(s: &str) -> String {
    s.split('_').map(|w| {
        let mut c = w.chars();
        match c.next() {
            None    => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }).collect::<Vec<_>>().join(" ")
}

/// Bump "1.0" → "1.1"
fn bump_version(v: &str) -> String {
    let parts: Vec<&str> = v.splitn(2, '.').collect();
    if parts.len() == 2 {
        if let (Ok(maj), Ok(min)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            return format!("{maj}.{}", min + 1);
        }
    }
    v.to_string()
}

/// Infer category from request keywords.
fn infer_category(request: &str) -> String {
    let r = request.to_lowercase();
    if r.contains("sofa") || r.contains("chair") || r.contains("couch") || r.contains("stool") {
        "seating"
    } else if r.contains("bed") || r.contains("mattress") || r.contains("headboard") {
        "bedroom"
    } else if r.contains("table") || r.contains("desk") {
        "tables"
    } else if r.contains("stair") || r.contains("door") || r.contains("window") || r.contains("arch") {
        "structure"
    } else if r.contains("lamp") || r.contains("light") || r.contains("pendant") {
        "lighting"
    } else if r.contains("tv") || r.contains("screen") || r.contains("monitor") {
        "electronics"
    } else if r.contains("plant") || r.contains("tree") || r.contains("garden") {
        "nature"
    } else if r.contains("kitchen") || r.contains("cabinet") || r.contains("island") {
        "kitchen"
    } else {
        "furniture"
    }.to_string()
}

// Tool Gen Job
// ── ToolGenProgress ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolGenPhase {
    Detecting,    // 0-10%
    WritingSpec,  // 10-40%
    Generating,   // 40-80%
    Ingesting,    // 80-95%
    Done,         // 100%
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGenProgress {
    pub job_id:  String,
    pub phase:   ToolGenPhase,
    pub percent: u8,
    pub message: String,
    pub error:   Option<String>,
}

impl ToolGenProgress {
    pub fn new(job_id: &str) -> Self {
        Self {
            job_id:  job_id.to_string(),
            phase:   ToolGenPhase::Detecting,
            percent: 0,
            message: "Starting...".to_string(),
            error:   None,
        }
    }

    pub fn update(&mut self, phase: ToolGenPhase, percent: u8, message: &str) {
        self.phase   = phase;
        self.percent = percent;
        self.message = message.to_string();
    }

    pub fn fail(&mut self, error: &str) {
        self.phase   = ToolGenPhase::Failed;
        self.error   = Some(error.to_string());
        self.message = "Failed".to_string();
    }
}

// ── ToolGenJob ────────────────────────────────────────────────────────────────
pub struct ToolGenJob {
    pub job_id:   String,
    pub request:  String,
    pub domain:   String,
    pub progress: Arc<Mutex<ToolGenProgress>>,
    pub result:   Arc<Mutex<Option<SpawnResult>>>,
    pub manifest: Arc<Mutex<JobManifest>>,
    pub cancel:   tokio_util::sync::CancellationToken,
}

/// Shared job store type — same pattern as AgentStore.
pub type JobStore = Arc<Mutex<HashMap<String, ToolGenJob>>>;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"
// TOOL_NAME: Curved Sectional Sofa
// FILE_STEM: curved_sectional_sofa
// CATEGORY: seating
// REQUEST: curved sectional sofa with chaise lounge
// BASE_TOOL: sofa
// STATUS: pending
// CREATED_AT: 2026-04-29T10:00:00Z

## Description
A curved sectional sofa with chaise lounge extension.

## Components
- Main section: 3 seats, cream fabric
- Chaise lounge: extended right side

## Dimensions
- Width: 2.8m
- Depth: 1.6m

## Materials
- primary: fabric_cream

## Three.js Approach
BoxGeometry segments for curve approximation.

## Styles
modern, curved, sectional

## Tags
sofa, sectional, chaise, curved, seating
"#;

    #[test]
    fn test_parse_spec() {
        let spec = ToolSpec::parse(SPEC).unwrap();
        println!("Tags: {:?}", spec.tags);
        println!("Styles: {:?}", spec.styles);
        assert_eq!(spec.tool_name, "Curved Sectional Sofa");
        assert_eq!(spec.file_stem, "curved_sectional_sofa");
        assert_eq!(spec.category,  "seating");
        assert_eq!(spec.base_tool, Some("sofa".to_string()));
        assert!(spec.tags.contains(&"sofa".to_string()));
        assert!(spec.styles.contains(&"modern".to_string()));
        assert!(spec.description.contains("curved sectional"));
    }

    #[test]
    fn test_parse_spec_no_base_tool() {
        let content = "// TOOL_NAME: TV Stand\n// FILE_STEM: tv_stand\n// BASE_TOOL:\n\
                       ## Description\nA TV stand.\n## Styles\nmodern\n## Tags\ntv, stand";
        let spec = ToolSpec::parse(content).unwrap();
        assert_eq!(spec.base_tool, None);
    }

    #[test]
    fn test_parse_spec_missing_required_fields() {
        let content = "// CATEGORY: seating\n";
        assert!(ToolSpec::parse(content).is_err());
    }

    #[test]
    fn test_extract_md_section() {
        let s = extract_md_section(SPEC, "Description");
        assert!(s.contains("curved sectional sofa"));
    }

    #[test]
    fn test_extract_md_section_missing() {
        assert!(extract_md_section(SPEC, "NonExistent").is_empty());
    }

    #[test]
    fn test_derive_file_stem() {
        assert_eq!(
            derive_file_stem("curved sectional sofa with chaise lounge"),
            "curved_sectional_sofa_chaise"
        );
        assert_eq!(
            derive_file_stem("spiral staircase between floors"),
            "spiral_staircase_between_floors"
        );
    }

    #[test]
    fn test_title_case() {
        assert_eq!(title_case("curved_sectional_sofa"), "Curved Sectional Sofa");
        assert_eq!(title_case("tv_stand"),              "Tv Stand");
        assert_eq!(title_case("spiral_staircase"),      "Spiral Staircase");
    }

    #[test]
    fn test_bump_version() {
        assert_eq!(bump_version("1.0"), "1.1");
        assert_eq!(bump_version("1.9"), "1.10");
        assert_eq!(bump_version("2.3"), "2.4");
        assert_eq!(bump_version("bad"), "bad");
    }

    #[test]
    fn test_infer_category() {
        assert_eq!(infer_category("curved sectional sofa"), "seating");
        assert_eq!(infer_category("add a TV stand"),        "electronics");
        assert_eq!(infer_category("spiral staircase"),      "structure");
        assert_eq!(infer_category("pendant lamp"),          "lighting");
        assert_eq!(infer_category("kitchen island"),        "kitchen");
        assert_eq!(infer_category("random object"),         "furniture");
    }

    #[test]
    fn test_typescript_generator_extension() {
        assert_eq!(TypeScriptGenerator.extension(), "ts");
    }

    #[test]
    fn test_rollback_created_file() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_tool.ts");
        fs::write(&path, "// new").unwrap();

        let mut manifest = JobManifest::new("job-1");
        manifest.track_created(path.clone());

        use crate::graph::structs::{DomainGraph, GraphId};
        use crate::graph::enums::Domain;
        let mut graph = DomainGraph::new(GraphId::new("t"), Domain::Custom("t".into()));

        assert!(path.exists());
        manifest.rollback(&mut graph);
        assert!(!path.exists());
    }

    #[test]
    fn test_rollback_modified_file() {
        let dir  = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.ts");
        fs::write(&path, "// original").unwrap();

        let mut manifest = JobManifest::new("job-2");
        manifest.track_modified(&path).unwrap();
        fs::write(&path, "// modified").unwrap();

        use crate::graph::structs::{DomainGraph, GraphId};
        use crate::graph::enums::Domain;
        let mut graph = DomainGraph::new(GraphId::new("t"), Domain::Custom("t".into()));

        manifest.rollback(&mut graph);
        assert_eq!(fs::read_to_string(&path).unwrap(), "// original");
    }

    #[test]
    fn test_manifest_tracks_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let p1  = dir.path().join("a.ts");
        let p2  = dir.path().join("b.ts");
        fs::write(&p1, "// a").unwrap();
        fs::write(&p2, "// b").unwrap();

        let mut manifest = JobManifest::new("job-3");
        manifest.track_created(p1);
        manifest.track_modified(&p2).unwrap();
        manifest.track_node("tool://architecture/test".to_string());

        assert_eq!(manifest.created_files.len(),  1);
        assert_eq!(manifest.modified_files.len(), 1);
        assert_eq!(manifest.created_nodes.len(),  1);
    }

    #[test]
    fn test_spawn_result_fields() {
        let r = SpawnResult {
            action:    "generated".to_string(),
            file_name: "tv_stand.ts".to_string(),
            file_path: PathBuf::from("generated/tv_stand.ts"),
            spec_path: Some(PathBuf::from("specs/tv_stand.md")),
            tool_name: "Tv Stand".to_string(),
            is_new:    true,
            job_id:    Some("job-abc".to_string()),
        };
        assert!(r.is_new);
        assert!(r.spec_path.is_some());
        assert_eq!(r.action, "generated");
    }
}