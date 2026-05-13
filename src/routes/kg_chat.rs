//! `POST /chat` — answers using the **SurrealDB** knowledge graph (vector seeds + graph hop
//! neighborhood). Does not read the in-memory `DomainGraph`.

use std::collections::HashSet;

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::authentication::AuthUser;
use crate::storage::surreal::SurrealNodeRow;

const SIM_TOP: usize = 36;
const SEED_K: usize = 6;
const BFS_DEPTH: usize = 2;
const NEIGHBOR_CAP: usize = 10;
const SEED_TEXT_CHARS: usize = 700;
const NEIGHBOR_PREVIEW_CHARS: usize = 220;

#[derive(Deserialize)]
pub struct KgChatRequest {
    pub question:   String,
    pub history:    Vec<HistoryMessage>,
    /// Repo-relative path prefix (e.g. codebase focus) — filters similarity seeds in Rust.
    #[serde(default)]
    pub focus_path: Option<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct HistoryMessage {
    pub role:    String,
    pub content: String,
}

#[derive(Serialize)]
pub struct KgChatResponse {
    pub answer:  String,
    pub sources: Vec<SourceNode>,
}

#[derive(Serialize)]
pub struct SourceNode {
    pub id:    String,
    pub page:  String,
    pub score: f32,
    pub text:  String,
}

fn truncate(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

fn display_path(meta: &std::collections::HashMap<String, String>) -> String {
    meta.get("path")
        .cloned()
        .or_else(|| meta.get("page").cloned())
        .unwrap_or_else(|| "?".to_string())
}

fn surreal_row_matches_path_prefix(row: &SurrealNodeRow, prefix: &str) -> bool {
    let p = prefix.trim().trim_end_matches('/').replace('\\', "/");
    if p.is_empty() {
        return true;
    }
    if let Some(path) = row.metadata.get("path") {
        let path = path.replace('\\', "/");
        if path == p || path.starts_with(&format!("{p}/")) {
            return true;
        }
    }
    row.source_uri.replace('\\', "/").contains(&p)
}

/// Pick top semantic seeds; if `focus_path` is set, prefer rows under that prefix (fallback to global).
fn select_seeds(
    rows:        Vec<SurrealNodeRow>,
    path_prefix: Option<&str>,
    seed_k:      usize,
) -> Vec<(SurrealNodeRow, f32)> {
    let trimmed = path_prefix.map(str::trim).filter(|s| !s.is_empty());
    let mut scored: Vec<(SurrealNodeRow, f32)> = rows
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let score = 1.0_f32 - (i as f32) * (1.0_f32 / SIM_TOP as f32).max(0.001);
            (r, score)
        })
        .collect();

    if let Some(pfx) = trimmed {
        let filtered: Vec<(SurrealNodeRow, f32)> = scored
            .iter()
            .filter(|(r, _)| surreal_row_matches_path_prefix(r, pfx))
            .take(seed_k)
            .cloned()
            .collect();
        if !filtered.is_empty() {
            return filtered;
        }
    }

    scored.truncate(seed_k);
    scored
}

fn row_heading(row: &SurrealNodeRow) -> String {
    let page = display_path(&row.metadata);
    let source = row
        .metadata
        .get("source")
        .cloned()
        .unwrap_or_default();
    format!("source={source} | page={page}")
}

pub async fn post_kg_chat(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Json(req): Json<KgChatRequest>,
) -> Result<Json<KgChatResponse>, (StatusCode, String)> {
    let question = req.question.trim().to_string();
    if question.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "question required".into()));
    }

    let query_vec = {
        let pipeline = state.pipeline.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "pipeline lock poisoned".into(),
            )
        })?;
        let mut ctx = pipeline.embed_ctx.lock().map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "embedding context lock poisoned".into(),
            )
        })?;
        ctx.embed(&question).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?
    };

    let rows = state
        .surreal_storage
        .similarity_search_nodes(user.id, &query_vec, SIM_TOP, 2)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Surreal similarity_search_nodes: {e}"),
            )
        })?;

    if rows.is_empty() {
        return Ok(Json(KgChatResponse {
            answer: "I could not find relevant content in your Surreal knowledge graph yet. \
                     Upload a PDF or video, add a twin note, or ingest content that persists to Surreal."
                .to_string(),
            sources: vec![],
        }));
    }

    let focus = req
        .focus_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let seeds = select_seeds(rows, focus, SEED_K);

    let mut intro = "Knowledge graph retrieval (SurrealDB): each section is a semantically matched chunk (seed) \
and other nodes within a short hop on graph relationships stored in Surreal (from your ingests).\n\
When the user asks about their project, codebase, or documents, prioritize concrete text from the chunks.\n"
        .to_string();
    if let Some(pfx) = focus {
        intro.push_str(&format!("Scope: seeds prefer repo path prefix `{pfx}`.\n"));
    }

    let mut parts: Vec<String> = vec![intro];
    let mut sources: Vec<SourceNode> = Vec::new();
    let mut seen_uris: HashSet<String> = HashSet::new();

    for (rank, (row, score)) in seeds.iter().enumerate() {
        let node_id = row.to_node().id;
        let id_short = node_id.to_string();
        let page = display_path(&row.metadata);
        let block_head = format!(
            "## Seed {} (rank {})\n{}\n{}\n",
            id_short.chars().take(8).collect::<String>(),
            rank + 1,
            row_heading(row),
            truncate(&row.source_text, SEED_TEXT_CHARS),
        );

        seen_uris.insert(row.source_uri.clone());

        let mut neighbor_lines: Vec<String> = Vec::new();
        match state.surreal_storage.bfs(&node_id, BFS_DEPTH).await {
            Ok(nb) => {
                for n in nb {
                    if seen_uris.contains(&n.source_uri) {
                        continue;
                    }
                    if neighbor_lines.len() >= NEIGHBOR_CAP {
                        break;
                    }
                    seen_uris.insert(n.source_uri.clone());
                    let p = display_path(&n.metadata);
                    let nid = n.to_node().id.to_string();
                    let short = nid.chars().take(8).collect::<String>();
                    neighbor_lines.push(format!(
                        "- neighbor {short} … page {p} | {}",
                        truncate(&n.source_text, NEIGHBOR_PREVIEW_CHARS).replace('\n', " ")
                    ));
                }
            }
            Err(e) => tracing::warn!("[POST /chat] Surreal bfs from {node_id}: {e}"),
        }

        let neigh_block = if neighbor_lines.is_empty() {
            "Graph neighborhood: (no linked nodes in Surreal within hop limit, or edges not stored)\n"
                .to_string()
        } else {
            format!("Graph neighborhood (Surreal hops ≤{BFS_DEPTH}):\n{}", neighbor_lines.join("\n"))
        };

        parts.push(format!("{block_head}{neigh_block}"));

        sources.push(SourceNode {
            id:    id_short,
            page,
            score: *score,
            text:  row.source_text.chars().take(120).collect(),
        });
    }

    let context = parts.join("\n");

    let mut messages: Vec<serde_json::Value> = req
        .history
        .iter()
        .map(|h| serde_json::json!({"role": h.role, "content": h.content}))
        .collect();
    messages.push(serde_json::json!({"role": "user", "content": question}));

    let system = format!(
        "You are a helpful assistant answering questions using the user's knowledge graph stored in SurrealDB.\n\
         The context lists semantically retrieved seed chunks and neighboring graph nodes reachable in Surreal.\n\
         Answer using ONLY this context. Be concise.\n\
         If the answer is not supported by the context, say \"I don't see that in the knowledge graph context.\"\n\n\
         KNOWLEDGE GRAPH CONTEXT:\n{context}"
    );

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &state.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 1024,
            "system": system,
            "messages": messages
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let answer = res["content"][0]["text"]
        .as_str()
        .unwrap_or("(no response)")
        .to_string();

    Ok(Json(KgChatResponse { answer, sources }))
}
