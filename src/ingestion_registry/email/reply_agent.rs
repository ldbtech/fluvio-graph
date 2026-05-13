//! Gmail reply agent: optional auto-reply using Surreal + network context selected by user prefs.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::app_state::AppState;
use crate::database::connections::get_connected_user_ids;
use crate::database::gmail_inbox_prefs;
use crate::database::gmail_reply_agent::{
    agent_message_processed, get_reply_agent_settings, mark_agent_processed, GmailAgentContextSources,
    GmailAgentSendMode, ProcessOutcomeDb,
};
use crate::database::users::{get_user_by_id, User};
use crate::ingestion_registry::email::client::gmail::{GmailClient, GmailClientError};
use crate::ingestion_registry::email::client::models::{GmailMessage, GmailThread};
use crate::ingestion_registry::email::{gmail_query};
use crate::storage::surreal::SurrealNodeRow;

const SIM_TOP_SELF: usize = 28;
const SIM_TOP_PEER: usize = 6;
const CONTEXT_CAP: usize = 22_000;
const THREAD_MSG_BODY_CAP: usize = 1_200;
const AUTO_SEND_MIN_CONFIDENCE: f32 = 0.78_f32;

// ── API response ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct GmailAgentCycleResponse {
    pub dry_run: bool,
    pub items: Vec<GmailAgentCycleItem>,
}

#[derive(Debug, Serialize)]
pub struct GmailAgentCycleItem {
    pub thread_id:              String,
    pub trigger_message_id:     String,
    pub outcome:                String,
    pub reply_proposal:         Option<String>,
    pub gmail_sent_message_id:  Option<String>,
    pub detail:                 Option<String>,
}

// ── Surreal row classification (aligned with Twin workspace heuristics) ───────────

fn surreal_row_workspace_library(n: &SurrealNodeRow) -> bool {
    let src = n
        .metadata
        .get("source")
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(
        src.as_str(),
        "pdf" | "email" | "gmail" | "codebase" | "video" | "architecture" | "tools"
    ) {
        return true;
    }
    let d = n.domain.as_str();
    matches!(d, "Pdf" | "Email" | "Codebase" | "Architecture") || d == r#"Custom("video")"#
}

fn row_matches_uploads(r: &SurrealNodeRow) -> bool {
    let src = r.metadata.get("source").map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    matches!(src.as_str(), "pdf" | "video")
}

fn row_matches_codebase(r: &SurrealNodeRow) -> bool {
    let src = r.metadata.get("source").map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    if src == "codebase" {
        return true;
    }
    let d = r.domain.to_ascii_lowercase();
    d.contains("codebase")
}

fn row_matches_email_ingest(r: &SurrealNodeRow) -> bool {
    let src = r.metadata.get("source").map(|s| s.to_ascii_lowercase()).unwrap_or_default();
    matches!(src.as_str(), "email" | "gmail")
}

fn row_matches_twin_notes(r: &SurrealNodeRow) -> bool {
    !surreal_row_workspace_library(r)
}

fn surreal_row_allowed(row: &SurrealNodeRow, p: &GmailAgentContextSources) -> bool {
    if p.uploads && row_matches_uploads(row) {
        return true;
    }
    if p.github_codebase && row_matches_codebase(row) {
        return true;
    }
    if p.ingested_email && row_matches_email_ingest(row) {
        return true;
    }
    if p.twin_notes && row_matches_twin_notes(row) {
        return true;
    }
    false
}

fn context_line(row: &SurrealNodeRow, max_chars: usize) -> String {
    let kind = row
        .metadata
        .get("kind")
        .map(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| row.metadata.get("source").map(|s| s.as_str()))
        .unwrap_or("chunk");
    let tag = row
        .metadata
        .get("filename")
        .or_else(|| row.metadata.get("title"))
        .map(|s| format!(" `{}`", s.replace('`', "'")))
        .unwrap_or_default();
    let excerpt: String = row.source_text.chars().take(max_chars).collect();
    format!("- [{kind}]{tag}\n  {excerpt}")
}

fn dedup_key(row: &SurrealNodeRow) -> String {
    format!("{}|{}", row.source_uri, row.domain)
}

fn embed_hint_vec(state: &AppState, hint: &str) -> Option<Vec<f32>> {
    let t = hint.trim();
    if t.is_empty() {
        return None;
    }
    let pipeline = state.pipeline.lock().ok()?;
    let mut ctx = pipeline.embed_ctx.lock().ok()?;
    ctx.embed(t).ok()
}

async fn build_context_pack(
    state: &AppState,
    viewer: &User,
    prefs: &GmailAgentContextSources,
    embedding_hint: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    if prefs.account_profile {
        parts.push(format!(
            "## Account (PostgreSQL)\n- Name: {}\n- Email: {}\n- Phone: {}",
            viewer.name,
            viewer.email.as_deref().unwrap_or("—"),
            viewer.phone.as_deref().unwrap_or("—"),
        ));
    }

    let want_self_surreal = prefs.uploads || prefs.github_codebase || prefs.ingested_email || prefs.twin_notes;
    if want_self_surreal {
        let qvec = embed_hint_vec(state, embedding_hint);
        let mut ordered: Vec<SurrealNodeRow> = Vec::new();
        let mut seen = HashSet::new();

        if let Some(ref v) = qvec {
            if let Ok(sim) =
                state
                    .surreal_storage
                    .similarity_search_nodes(viewer.id, v.as_slice(), SIM_TOP_SELF, 2)
                    .await
            {
                for row in sim {
                    if !surreal_row_allowed(&row, prefs) {
                        continue;
                    }
                    let k = dedup_key(&row);
                    if seen.insert(k) {
                        ordered.push(row);
                    }
                }
            }
        }

        if let Ok(all) = state.surreal_storage.get_user_nodes(viewer.id, None, 2).await {
            for row in &all {
                if row.metadata.get("dashboard_doc_anchor").map(|s| s.as_str()) == Some("1")
                    && surreal_row_allowed(row, prefs)
                {
                    let k = dedup_key(row);
                    if seen.insert(k) {
                        ordered.push(row.clone());
                    }
                }
            }
            if ordered.len() < 12 {
                let mut tail: Vec<SurrealNodeRow> = all
                    .iter()
                    .filter(|r| surreal_row_workspace_library(r) && surreal_row_allowed(r, prefs))
                    .cloned()
                    .collect();
                tail.sort_by(|a, b| a.source_uri.cmp(&b.source_uri));
                for row in tail {
                    let k = dedup_key(&row);
                    if seen.insert(k) {
                        ordered.push(row);
                    }
                    if ordered.len() >= 48 {
                        break;
                    }
                }
            }
            if ordered.is_empty() {
                for row in &all {
                    if surreal_row_allowed(row, prefs) {
                        ordered.push(row.clone());
                    }
                    if ordered.len() >= 32 {
                        break;
                    }
                }
            }
        }

        if !ordered.is_empty() {
            let lines: Vec<String> = ordered
                .into_iter()
                .map(|row| context_line(&row, 520))
                .collect();
            parts.push(format!(
                "## Workspace knowledge (filtered by your source toggles)\n{}",
                lines.join("\n\n")
            ));
        }
    }

    if prefs.network_connections {
        let mut roster = Vec::new();
        if let Ok(peers) = get_connected_user_ids(&state.pg_pool, viewer.id).await {
            let qvec = embed_hint_vec(state, embedding_hint);
            for (peer_id, zone) in peers.iter().take(16) {
                let line = match get_user_by_id(&state.pg_pool, *peer_id).await {
                    Ok(Some(p)) => {
                        format!(
                            "- {} ({}) · connection zone {}",
                            p.name,
                            p.email.clone().unwrap_or_else(|| "?".into()),
                            zone
                        )
                    }
                    Ok(None) => format!("- (unknown user {peer_id})"),
                    Err(_) => continue,
                };
                roster.push(line);
                if let Some(ref v) = qvec {
                    if let Ok(rows) =
                        state
                            .surreal_storage
                            .similarity_search_nodes(*peer_id, v.as_slice(), SIM_TOP_PEER, *zone)
                            .await
                    {
                        for row in rows {
                            roster.push(format!(
                                "  · peer excerpt: {}",
                                context_line(&row, 380)
                            ));
                        }
                    }
                }
            }
        }
        if !roster.is_empty() {
            parts.push(format!(
                "## Network (NFC connections + optional matching peer graph snippets)\n{}",
                roster.join("\n")
            ));
        }
    }

    let joined = parts.join("\n\n");
    joined.chars().take(CONTEXT_CAP).collect()
}

#[derive(Debug, Deserialize)]
struct LlmDecision {
    #[serde(default)]
    should_reply:          bool,
    #[serde(default)]
    confidence:            f32,
    #[serde(default)]
    reply_body_plain:      String,
    #[serde(default)]
    reason:                String,
}

fn extract_brace_json(s: &str) -> Option<&str> {
    let t = s.trim();
    let i = t.find('{')?;
    let j = t.rfind('}')?;
    if j > i {
        Some(&t[i..=j])
    } else {
        None
    }
}

async fn anthropic_reply_decision(
    api_key: &str,
    thread_block: &str,
    context_pack: &str,
    mailbox_owner_email: Option<&str>,
) -> anyhow::Result<LlmDecision> {
    let sys = concat!(
        "You decide whether to compose an outbound email reply.\n",
        "Return ONLY valid compact JSON with keys:\n",
        "should_reply (boolean),\n",
        "confidence (0.0–1.0, how confident a human would be sending this),\n",
        "reply_body_plain (plain text body only; no mail headers),\n",
        "reason (one short phrase).\n",
        "\n",
        "Rules:\n",
        "- Self-addressed mail (From and To are the mailbox owner’s address, or reminders the user emailed to themselves) is NORMAL. ",
        "If the body asks a question or requests status, an update, or a decision, set should_reply true and draft a concise, professional reply the owner could send (e.g. project status).\n",
        "- Do NOT refuse solely because From looks like the same person as To.\n",
        "- Set should_reply false for true no-reply / automated transactional mail.\n",
        "- If unsure, prefer should_reply false with low confidence.",
    );

    let mut user = String::new();
    if let Some(mb) = mailbox_owner_email.filter(|s| !s.trim().is_empty()) {
        user.push_str(&format!("MAILBOX_OWNER_EMAIL (connected Gmail / account): {mb}\n\n"));
    }
    user.push_str(&format!(
        "THREAD (newest toward bottom):\n{thread_block}\n\nKNOWLEDGE CONTEXT YOU MAY USE (may be incomplete):\n{context_pack}",
    ));

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-5",
            "max_tokens": 1_024,
            "system": sys,
            "messages": [{"role": "user", "content": user}]
        }))
        .send()
        .await?;
    let v: serde_json::Value = res.json().await?;
    let txt = v["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing Anthropic response text"))?;
    let json_part = extract_brace_json(txt).ok_or_else(|| anyhow::anyhow!("no JSON in model output"))?;
    serde_json::from_str(json_part).map_err(|e| anyhow::anyhow!("decode decision JSON: {e}"))
}

fn sort_messages_thread(thread: &GmailThread) -> Vec<&GmailMessage> {
    let mut v: Vec<_> = thread.messages.iter().collect();
    v.sort_by_key(|m| m.internal_date.unwrap_or(0));
    v
}

fn format_thread_prompt(thread: &GmailThread) -> String {
    let mut blocks = Vec::new();
    for m in sort_messages_thread(thread) {
        let from = m.from().unwrap_or("?");
        let sub = m.subject().unwrap_or("(no subject)");
        let dt = m.date().unwrap_or("?");
        let to = m.to().unwrap_or("(not shown)");
        let body = m
            .plain_text_body()
            .unwrap_or_else(|| m.snippet.clone().unwrap_or_default())
            .chars()
            .take(THREAD_MSG_BODY_CAP)
            .collect::<String>();
        blocks.push(format!(
            "---\nDate: {dt}\nFrom: {from}\nTo: {to}\nSubject: {sub}\n\n{body}",
        ));
    }
    blocks.join("\n")
}

fn embedding_hint_for_thread(trigger: &GmailMessage, thread: &GmailThread) -> String {
    let sub = trigger.subject().unwrap_or("");
    let sn = trigger.snippet.as_deref().unwrap_or("");
    let last_plain = sort_messages_thread(thread)
        .into_iter()
        .rev()
        .find_map(|m| {
            let p = m.plain_text_body()?;
            if p.trim().is_empty() {
                None
            } else {
                Some(p)
            }
        })
        .unwrap_or_default();
    let plain_trim: String = last_plain.chars().take(2800).collect();
    format!("{sub}\n\n{sn}\n\n{plain_trim}")
}

fn header_one_line(raw: Option<&str>, fallback: &str) -> String {
    let s = raw.unwrap_or(fallback);
    let line = s.lines().next().unwrap_or_default();
    line.chars()
        .filter(|c| *c != '\r' && *c != '\n')
        .take(220)
        .collect::<String>()
}

fn is_me(msg: &GmailMessage, me: Option<&str>) -> bool {
    let Some(me_raw) = me else {
        return false;
    };
    let Some(from_h) = msg.from() else {
        return false;
    };
    let Some(parsed) = gmail_query::extract_email_address(from_h) else {
        return false;
    };
    gmail_query::email_identities_equivalent(&parsed, me_raw)
}

fn last_peer_message<'a>(sorted: &[&'a GmailMessage], me: Option<&str>) -> Option<&'a GmailMessage> {
    sorted.iter().rev().find(|m| !is_me(*m, me)).copied()
}

fn compose_rfc822_reply(
    me_addr: &str,
    peer_addr: &str,
    peer_name_hint: Option<&str>,
    subject: &str,
    in_reply_to: Option<&str>,
    references: Option<&str>,
    body_plain: &str,
) -> String {
    let t = subject.trim();
    let sub = if t.len() >= 3 && t[..3].eq_ignore_ascii_case("re:") {
        t.to_string()
    } else {
        format!("Re: {t}")
    };
    let to_line = peer_name_hint
        .filter(|n| !n.is_empty())
        .map(|n| format!("{n} <{peer_addr}>"))
        .unwrap_or_else(|| peer_addr.to_string());

    let mut h = Vec::new();
    h.push(format!("MIME-Version: 1.0"));
    h.push(format!("Subject: {}", sub));
    h.push(format!("From: {}", me_addr));
    h.push(format!("To: {}", to_line));
    h.push(format!("Content-Type: text/plain; charset=UTF-8"));
    if let Some(m) = in_reply_to {
        h.push(format!("In-Reply-To: {m}"));
    }
    if let Some(r) = references {
        h.push(format!("References: {}", r.trim()));
    }

    format!("{}\r\n\r\n{}", h.join("\r\n"), body_plain.trim())
}

pub fn map_gmail_err(e: GmailClientError) -> (axum::http::StatusCode, String) {
    match e {
        GmailClientError::NotAuthenticated => (
            axum::http::StatusCode::FORBIDDEN,
            "Gmail not connected — connect Gmail.".to_string(),
        ),
        GmailClientError::TokenRefresh(msg) => (axum::http::StatusCode::BAD_GATEWAY, msg),
        GmailClientError::ApiError { status, body } => (
            axum::http::StatusCode::BAD_GATEWAY,
            format!(
                "Gmail API HTTP {status}: {}",
                body.chars().take(500).collect::<String>()
            ),
        ),
        GmailClientError::RateLimited => (axum::http::StatusCode::TOO_MANY_REQUESTS, e.to_string()),
        GmailClientError::Deserialize(err) => (axum::http::StatusCode::BAD_GATEWAY, err.to_string()),
        GmailClientError::Http(msg) => (axum::http::StatusCode::BAD_GATEWAY, msg),
    }
}

pub async fn run_gmail_agent_cycle(
    state: &AppState,
    user: &User,
    dry_run: bool,
    max_candidates: u32,
) -> Result<GmailAgentCycleResponse, (axum::http::StatusCode, String)> {
    let prefs_row = get_reply_agent_settings(&state.pg_pool, user.id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?;

    let mut resp = GmailAgentCycleResponse {
        dry_run,
        items: Vec::new(),
    };

    // Dry runs skip idempotency writes; live runs process automatically for Gmail-connected users.

    if state.api_key.trim().is_empty() {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "Server missing Anthropic API key — cannot run reply agent.".into(),
        ));
    }

    let max_c = max_candidates.clamp(1, 12);
    let focus = gmail_inbox_prefs::list_focus_senders(&state.pg_pool, user.id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            )
        })?;
    let gmail_q = gmail_query::inbox_recent_list_q(&focus);

    let mut client = GmailClient::for_user(&state.pg_pool, user.id)
        .await
        .map_err(map_gmail_err)?;

    let profile = client.get_user_profile().await.map_err(map_gmail_err)?;
    let my_addr = profile
        .email_address
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| user.email.as_deref());
    let me_lc = my_addr.map(|s| s.to_ascii_lowercase());

    let candidates = client
        .inbox_recent_summaries(max_c, &gmail_q)
        .await
        .map_err(map_gmail_err)?;

    for trigger in candidates {
        if agent_message_processed(&state.pg_pool, user.id, &trigger.id)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    e.to_string(),
                )
            })? {
            continue;
        }

        if is_me(&trigger, me_lc.as_deref()) {
            if !dry_run {
                mark_agent_processed(
                    &state.pg_pool,
                    user.id,
                    &trigger.id,
                    ProcessOutcomeDb::Skipped,
                    Some("from_own_gmail_address"),
                    None,
                    None,
                    None,
                )
                .await
                .ok();
            }
            resp.items.push(GmailAgentCycleItem {
                thread_id:             trigger.thread_id.clone(),
                trigger_message_id:    trigger.id.clone(),
                outcome:               "skipped_from_own_address".into(),
                reply_proposal:        None,
                gmail_sent_message_id: None,
                detail: Some(
                    "This inbox row lists your address as the sender — use mail from someone else for normal replies."
                        .into(),
                ),
            });
            continue;
        }

        let Ok(thread) = client.get_thread(&trigger.thread_id).await else {
            if !dry_run {
                mark_agent_processed(
                    &state.pg_pool,
                    user.id,
                    &trigger.id,
                    ProcessOutcomeDb::Error,
                    Some("thread_fetch_failed"),
                    None,
                    None,
                    None,
                )
                .await
                .ok();
            }
            continue;
        };

        let sorted = sort_messages_thread(&thread);
        let peer = match last_peer_message(&sorted, me_lc.as_deref()) {
            Some(p) => p,
            None => {
                if !dry_run {
                    mark_agent_processed(
                        &state.pg_pool,
                        user.id,
                        &trigger.id,
                        ProcessOutcomeDb::Skipped,
                        Some("thread_only_me"),
                        None,
                        None,
                        None,
                    )
                    .await
                    .ok();
                }
                resp.items.push(GmailAgentCycleItem {
                    thread_id:             trigger.thread_id.clone(),
                    trigger_message_id:    trigger.id.clone(),
                    outcome:               "skipped_thread_only_from_me".into(),
                    reply_proposal:        None,
                    gmail_sent_message_id: None,
                    detail:                None,
                });
                continue;
            }
        };

        let hint = embedding_hint_for_thread(&trigger, &thread);
        let ctx_pack = build_context_pack(state, user, &prefs_row.context_sources, &hint).await;
        let thread_block = format_thread_prompt(&thread);

        let mailbox_for_llm = my_addr.filter(|s| !s.trim().is_empty());

        let llm_out = anthropic_reply_decision(
            &state.api_key,
            &thread_block,
            &ctx_pack,
            mailbox_for_llm,
        )
            .await
            .unwrap_or_else(|e| LlmDecision {
                should_reply: false,
                confidence:   0.0,
                reply_body_plain: "".into(),
                reason: format!("llm_error: {e}"),
            });

        if !llm_out.should_reply || llm_out.reply_body_plain.trim().is_empty() {
            if !dry_run {
                mark_agent_processed(
                    &state.pg_pool,
                    user.id,
                    &trigger.id,
                    ProcessOutcomeDb::Skipped,
                    Some(llm_out.reason.as_str()),
                    None,
                    None,
                    None,
                )
                .await
                .ok();
            }
            resp.items.push(GmailAgentCycleItem {
                thread_id:             trigger.thread_id.clone(),
                trigger_message_id:    trigger.id.clone(),
                outcome:               "skipped_llm_decision".into(),
                reply_proposal:        None,
                gmail_sent_message_id: None,
                detail:                Some(llm_out.reason),
            });
            continue;
        }

        let peer_addr_raw = gmail_query::extract_email_address(peer.from().unwrap_or("")).unwrap_or_default();
        if peer_addr_raw.is_empty() {
            if !dry_run {
                mark_agent_processed(
                    &state.pg_pool,
                    user.id,
                    &trigger.id,
                    ProcessOutcomeDb::Skipped,
                    Some("no_peer_address"),
                    None,
                    None,
                    None,
                )
                .await
                .ok();
            }
            resp.items.push(GmailAgentCycleItem {
                thread_id:             trigger.thread_id.clone(),
                trigger_message_id:    trigger.id.clone(),
                outcome:               "skipped_no_peer_address".into(),
                reply_proposal:        Some(llm_out.reply_body_plain.clone()),
                gmail_sent_message_id: None,
                detail:                None,
            });
            continue;
        }

        let me_header = header_one_line(
            profile
                .email_address
                .as_deref()
                .or_else(|| user.email.as_deref())
                .filter(|s| !s.trim().is_empty()),
            "?@invalid",
        );
        let in_reply_to = peer.message_id_header();

        let rfc822 = compose_rfc822_reply(
            &me_header,
            &peer_addr_raw,
            None,
            header_one_line(trigger.subject(), peer.subject().unwrap_or("(no subject)")).as_str(),
            in_reply_to,
            None,
            &llm_out.reply_body_plain,
        );

        let send_ok = prefs_row.send_mode == GmailAgentSendMode::AutoWhenConfident && llm_out.confidence >= AUTO_SEND_MIN_CONFIDENCE;

        if dry_run {
            resp.items.push(GmailAgentCycleItem {
                thread_id:             trigger.thread_id.clone(),
                trigger_message_id:    trigger.id.clone(),
                outcome:               "dry_run_proposal".into(),
                reply_proposal:        Some(llm_out.reply_body_plain),
                gmail_sent_message_id: None,
                detail:                Some(llm_out.reason),
            });
            continue;
        }

        if !send_ok {
            let subject_hint = trigger.subject().map(|s| s.chars().take(512).collect::<String>());
            mark_agent_processed(
                &state.pg_pool,
                user.id,
                &trigger.id,
                ProcessOutcomeDb::DraftOnly,
                Some(llm_out.reason.as_str()),
                Some(llm_out.reply_body_plain.as_str()),
                Some(trigger.thread_id.as_str()),
                subject_hint.as_deref(),
            )
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    e.to_string(),
                )
            })?;
            resp.items.push(GmailAgentCycleItem {
                thread_id:             trigger.thread_id.clone(),
                trigger_message_id:    trigger.id.clone(),
                outcome:               "saved_draft_only".into(),
                reply_proposal:        Some(llm_out.reply_body_plain),
                gmail_sent_message_id: None,
                detail:                Some(llm_out.reason),
            });
            continue;
        }

        match client.send_rfc822(Some(&trigger.thread_id), &rfc822).await {
            Ok(sent) => {
                mark_agent_processed(
                    &state.pg_pool,
                    user.id,
                    &trigger.id,
                    ProcessOutcomeDb::Sent,
                    Some("auto_sent"),
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|e| {
                    (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        e.to_string(),
                    )
                })?;
                resp.items.push(GmailAgentCycleItem {
                    thread_id:             trigger.thread_id.clone(),
                    trigger_message_id:    trigger.id.clone(),
                    outcome:               "sent".into(),
                    reply_proposal:        Some(llm_out.reply_body_plain),
                    gmail_sent_message_id: Some(sent.id),
                    detail:                Some(llm_out.reason),
                });
            }
            Err(e) => {
                let msg = e.to_string();
                let subject_hint = trigger.subject().map(|s| s.chars().take(512).collect::<String>());
                mark_agent_processed(
                    &state.pg_pool,
                    user.id,
                    &trigger.id,
                    ProcessOutcomeDb::Error,
                    Some(msg.as_str()),
                    Some(llm_out.reply_body_plain.as_str()),
                    Some(trigger.thread_id.as_str()),
                    subject_hint.as_deref(),
                )
                .await
                .ok();
                resp.items.push(GmailAgentCycleItem {
                    thread_id:             trigger.thread_id.clone(),
                    trigger_message_id:    trigger.id.clone(),
                    outcome:               "send_failed".into(),
                    reply_proposal:        Some(llm_out.reply_body_plain),
                    gmail_sent_message_id: None,
                    detail:                Some(msg),
                });
            }
        }
    }

    Ok(resp)
}

/// Background scheduler: inbox pass for every user with Gmail OAuth in Postgres.
pub async fn run_gmail_agent_auto_poll_tick(state: &AppState) {
    if state.api_key.trim().is_empty() {
        return;
    }
    let user_ids = match crate::database::gmail_reply_agent::list_user_ids_with_gmail_credentials(&state.pg_pool).await
    {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!("[gmail agent auto-poll] list users failed: {e}");
            return;
        }
    };
    if user_ids.is_empty() {
        return;
    }
    for uid in user_ids {
        let user = match get_user_by_id(&state.pg_pool, uid).await {
            Ok(Some(u)) => u,
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!("[gmail agent auto-poll] load user {}: {}", uid, e);
                continue;
            }
        };
        match run_gmail_agent_cycle(state, &user, false, 5).await {
            Ok(resp) => {
                if !resp.items.is_empty() {
                    let outcomes: Vec<&str> = resp.items.iter().map(|i| i.outcome.as_str()).collect();
                    tracing::debug!(
                        target: "gmail_agent_auto_poll",
                        user_id = %uid,
                        outcomes = ?outcomes,
                        "completed auto-poll cycle"
                    );
                }
            }
            Err((st, msg)) => {
                tracing::warn!(
                    "[gmail agent auto-poll] user={} HTTP {} {}",
                    uid,
                    st.as_u16(),
                    msg,
                );
            }
        }
    }
}
