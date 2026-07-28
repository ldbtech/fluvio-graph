//! Query resolvers for fluvio-twin.

use async_graphql::*;
use uuid::Uuid;

use crate::server::AppState;
use crate::graphql::types::*;
use fluvio_twin_core::llm::{
    build_context_from_seeds, build_system_prompt,
    SEED_K, SIM_TOP_K, BFS_DEPTH,
};
use fluvio_llm::types::Message;

pub struct QueryRoot;

#[Object(name = "Query")]
impl QueryRoot {

    // ── Chat ──────────────────────────────────────────────────────────────────

    /// Ask a question over the user's knowledge graph.
    ///
    /// Pipeline:
    ///   1. Semantic search in fluvio-graph (top-K nodes)
    ///   2. BFS expansion for each seed (graph context)
    ///   3. Assemble knowledge context
    ///   4. Call Claude with context + question
    ///   5. Return grounded answer + sources
    async fn chat(
        &self,
        ctx:          &Context<'_>,
        question:     String,
        history:      Option<Vec<GqlChatMessage>>,
        top_k:        Option<i32>,
        zone:         Option<i32>,
        workspace_id: Option<String>,
    ) -> Result<GqlChatResponse> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let top_k   = top_k.unwrap_or(SIM_TOP_K as i32) as usize;
        let zone    = zone.unwrap_or(0) as i16;

        if question.trim().is_empty() {
            return Err(Error::new("question cannot be empty"));
        }

        // 1. Semantic search
        let search_results = state.graph_client
            .search(user_id, &question, top_k, zone, workspace_id.as_deref())
            .await
            .map_err(|e| Error::new(format!("search failed: {e}")))?;

        if search_results.is_empty() {
            return Ok(GqlChatResponse {
                answer: "I couldn't find relevant content in your knowledge graph yet. \
                         Try uploading a PDF or ingesting some text first.".to_string(),
                sources: vec![],
            });
        }

        // 2. Take top SEED_K results + BFS expand each
        let seeds_slice = &search_results[..search_results.len().min(SEED_K)];
        let mut seeds_with_neighbors = Vec::new();

        for seed in seeds_slice {
            let neighbors = state.graph_client
                .neighbors(user_id, &seed.id, BFS_DEPTH)
                .await
                .unwrap_or_default();
            seeds_with_neighbors.push((seed.clone(), neighbors));
        }

        // 3. Build context
        let user_name = "you"; // will be real name when fluvio-auth is wired
        let (context, context_sources) = build_context_from_seeds(
            &seeds_with_neighbors,
            user_name,
        );

        // 4. Assemble messages
        let mut messages: Vec<Message> = history
            .unwrap_or_default()
            .into_iter()
            .filter(|m| !m.content.trim().is_empty())
            .map(|m| Message { role: m.role, content: m.content })
            .collect();
        messages.push(Message {
            role:    "user".to_string(),
            content: question,
        });

        let system = build_system_prompt(&context, user_name, true);

        // 5. Resolve the caller's LLM provider (BYOK connection, or this
        // deployment's env-configured fallback) and call it.
        let provider_cfg = state.llm_resolver.resolve(user_id, None, None)
            .await
            .map_err(|e| Error::new(format!("no LLM provider available: {e}")))?;
        let answer = fluvio_llm::chat::chat(&provider_cfg, &system, &messages)
            .await
            .map_err(|e| Error::new(format!("LLM error: {e}")))?;

        let sources: Vec<GqlSource> = context_sources.into_iter().map(|s| GqlSource {
            id:    s.id,
            page:  s.page,
            score: s.score as f64,
            text:  s.text,
        }).collect();

        Ok(GqlChatResponse { answer, sources })
    }

    // ── Documents ─────────────────────────────────────────────────────────────

    /// List all documents (graph nodes) in the user's knowledge graph.
    async fn documents(
        &self,
        ctx:          &Context<'_>,
        zone:         Option<i32>,
        workspace_id: Option<String>,
    ) -> Result<Vec<GqlDocument>> {
        let state   = ctx.data::<AppState>()?;
        let user_id = extract_user_id(ctx)?;
        let zone    = zone.unwrap_or(0) as i16;

        let nodes = state.graph_client
            .nodes(user_id, zone, workspace_id.as_deref())
            .await
            .map_err(|e| Error::new(e.to_string()))?;

        let docs = nodes.into_iter().map(|n| {
            let title_raw = n.metadata.iter()
                .find(|(k, _)| k == "repo" || k == "filename" || k == "title")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| n.domain.clone());

            let title = get_clean_title(&n.domain, &title_raw, &n.source_uri);

            let kind = n.metadata.iter()
                .find(|(k, _)| k == "kind")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "chunk".to_string());

            GqlDocument {
                id:      n.id,
                title,
                kind,
                domain:  n.domain,
                excerpt: n.source_text.chars().take(200).collect(),
                zone:    n.zone,
            }
        }).collect();

        Ok(docs)
    }
}

fn get_clean_title(domain: &str, title: &str, source_uri: &str) -> String {
    if domain.to_uppercase() == "CODEBASE" {
        if let Some(github_idx) = source_uri.find("github://") {
            let sub = &source_uri[github_idx..];
            let parts: Vec<&str> = sub.split('/').collect();
            if parts.len() >= 4 {
                return format!("{}/{}", parts[2], parts[3]);
            }
        }
    }
    if title != "CODEBASE" && title != "CUSTOM" && !title.is_empty() {
        title.to_string()
    } else {
        domain.to_string()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new(
            "x-user-id header missing — request must go through the gateway"
        ))
}