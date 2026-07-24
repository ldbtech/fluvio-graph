//! Search and chat workflows for group knowledge graph.

use uuid::Uuid;
use reqwest::Client;
use serde_json::json;
use anyhow::Context;

use crate::clients::{DatabaseClient, GraphClient};
use crate::clients::graph_client::GraphNode;
use crate::policy::access;

pub const SEED_K:   usize = 6;
pub const BFS_DEPTH: usize = 2;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub node:  GraphNode,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub answer:  String,
    pub sources: Vec<ChatSource>,
}

#[derive(Debug, Clone)]
pub struct ChatSource {
    pub id:    String,
    pub score: f32,
    pub text:  String,
}

/// Search approved nodes in a group by semantic similarity.
pub async fn search_group(
    group_id:  &str,
    caller_id: Uuid,
    query:     &str,
    top_k:     usize,
    db:        &DatabaseClient,
    graph:     &GraphClient,
) -> anyhow::Result<Vec<SearchResult>> {
    let caller_str = caller_id.to_string();

    // Verify membership
    let member = db.get_member(group_id, &caller_str).await?
        .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    access::can_query(&member.role)?;

    // Search approved nodes only (filtered inside graph_client)
    let nodes = graph.search_group(caller_id, query, group_id, top_k).await?;

    Ok(nodes.into_iter().map(|n| {
        let score = n.score;
        SearchResult { node: n, score }
    }).collect())
}

/// Chat over the group's approved knowledge graph.
/// Uses RAG: search → BFS expand → assemble context → Claude.
pub async fn group_chat(
    group_id:     &str,
    caller_id:    Uuid,
    question:     &str,
    history:      Vec<(String, String)>,  // (role, content)
    anthropic_key: &str,
    db:           &DatabaseClient,
    graph:        &GraphClient,
) -> anyhow::Result<ChatResponse> {
    let caller_str = caller_id.to_string();

    // Verify membership
    let member = db.get_member(group_id, &caller_str).await?
        .ok_or_else(|| anyhow::anyhow!("you are not a member of this group"))?;

    access::can_query(&member.role)?;

    // Get group info for context
    let group_name = format!("group:{group_id}");

    // Semantic search → top seeds
    let seeds = graph.search_group(
        caller_id, question, group_id, SEED_K
    ).await?;

    if seeds.is_empty() {
        return Ok(ChatResponse {
            answer: format!(
                "No approved knowledge found in this group yet. \
                 Contribute some documents and have an owner approve them first."
            ),
            sources: vec![],
        });
    }

    // BFS expand each seed
    let mut context_parts = vec![
        format!("Company brain knowledge for group: {group_name}\n")
    ];
    let mut sources = vec![];

    for (rank, seed) in seeds.iter().enumerate() {
        let neighbors = graph.neighbors(caller_id, &seed.id, BFS_DEPTH)
            .await.unwrap_or_default();

        let seed_text: String = seed.source_text.chars().take(700).collect();
        context_parts.push(format!(
            "## Source {} (score: {:.3})\n{}\n",
            rank + 1, seed.score, seed_text
        ));

        for neighbor in neighbors.iter().take(5) {
            let preview: String = neighbor.source_text.chars().take(200).collect();
            context_parts.push(format!("  → related: {preview}\n"));
        }

        sources.push(ChatSource {
            id:    seed.id.clone(),
            score: seed.score,
            text:  seed.source_text.chars().take(120).collect(),
        });
    }

    let context = context_parts.join("\n");

    // Build messages
    let mut messages: Vec<serde_json::Value> = history.iter()
        .filter(|(_, c)| !c.trim().is_empty())
        .map(|(role, content)| json!({ "role": role, "content": content }))
        .collect();
    messages.push(json!({ "role": "user", "content": question }));

    // Later instead of source isnt there , we will use a way to figure out which digital twin that is responsible of specific things.
    let system = format!(
        "You are the AI assistant for a collaborative company knowledge graph.\n\
         Answer questions using ONLY the KNOWLEDGE CONTEXT below.\n\
         The knowledge has been curated and approved by group owners — treat it as authoritative.\n\
         Be concise and cite which source you're drawing from.\n\
         If the answer is not in the context, say so clearly.\n\n\
         KNOWLEDGE CONTEXT:\n{context}"
    );

    // Call Claude
    let client = Client::new();
    let resp: serde_json::Value = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key",         anthropic_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type",      "application/json")
        .json(&json!({
            "model":      "claude-sonnet-4-20250514",
            "max_tokens": 2048,
            "system":     system,
            "messages":   messages,
        }))
        .send()
        .await
        .context("failed to reach Anthropic API")?
        .json()
        .await
        .context("failed to parse Anthropic response")?;

    let answer = resp["content"][0]["text"]
        .as_str()
        .unwrap_or("No response generated.")
        .to_string();

    Ok(ChatResponse { answer, sources })
}