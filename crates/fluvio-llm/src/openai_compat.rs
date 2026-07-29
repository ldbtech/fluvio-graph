//! OpenAI chat-completions wire format — serves OpenAI itself, Ollama (which
//! ships an OpenAI-compatible endpoint), and any other self-hosted model that
//! speaks the same shape. Ollama typically needs no API key; OpenAI and most
//! others do.

use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Context;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::types::{Provider, ProviderConfig, Message};

/// Always ends in `/v1` — user-supplied `base_url`s (e.g. a bare
/// `http://host:11434`, as Ollama's own docs show it) commonly omit it, and
/// appending `/chat/completions` directly to those 404s instead of erroring
/// clearly (Ollama returns a plain-text 404 body, which then fails to parse
/// as JSON one layer up — a confusing error for what's really a URL bug).
fn base_url(cfg: &ProviderConfig) -> String {
    let raw = match &cfg.base_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => match cfg.provider {
            Provider::OpenAi => "https://api.openai.com".to_string(),
            // Ollama requires an explicit base_url (enforced by the DB
            // constraint and the connect mutation) — this default only
            // covers the common "running alongside this engine" case.
            Provider::Ollama => "http://localhost:11434".to_string(),
            _ => unreachable!("openai_compat only dispatches OpenAi/Ollama"),
        },
    };
    if raw.ends_with("/v1") { raw } else { format!("{raw}/v1") }
}

fn build_messages(system: &str, messages: &[Message]) -> Vec<Value> {
    let mut out = vec![json!({ "role": "system", "content": system })];
    out.extend(
        messages.iter()
            .filter(|m| !m.content.trim().is_empty())
            .map(|m| json!({ "role": m.role, "content": m.content }))
    );
    out
}

fn apply_auth(req: reqwest::RequestBuilder, cfg: &ProviderConfig) -> reqwest::RequestBuilder {
    match cfg.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
        Some(key) => req.bearer_auth(key),
        None      => req, // Ollama and similar local endpoints commonly need no auth
    }
}

pub async fn chat(
    cfg:      &ProviderConfig,
    system:   &str,
    messages: &[Message],
) -> anyhow::Result<String> {
    let client = Client::new();
    let url    = format!("{}/chat/completions", base_url(cfg));

    let req = client.post(&url).json(&json!({
        "model":    cfg.model(),
        "messages": build_messages(system, messages),
    }));

    let resp: Value = apply_auth(req, cfg)
        .send()
        .await
        .context("failed to reach OpenAI-compatible endpoint")?
        .json()
        .await
        .context("failed to parse OpenAI-compatible response")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("OpenAI-compatible endpoint error: {err}");
    }

    let answer = resp["choices"][0]["message"]["content"]
        .as_str()
        .context("no text in OpenAI-compatible response")?
        .to_string();

    Ok(answer)
}

pub async fn chat_streaming(
    cfg:      ProviderConfig,
    system:   String,
    messages: Vec<Message>,
    tx:       mpsc::Sender<anyhow::Result<String>>,
) {
    let client = Client::new();
    let url    = format!("{}/chat/completions", base_url(&cfg));

    let req = client.post(&url).json(&json!({
        "model":    cfg.model(),
        "stream":   true,
        "messages": build_messages(&system, &messages),
    }));

    let res = match apply_auth(req, &cfg).send().await {
        Ok(r)  => r,
        Err(e) => { let _ = tx.send(Err(anyhow::anyhow!(e.to_string()))).await; return; }
    };

    if !res.status().is_success() {
        let txt = res.text().await.unwrap_or_default();
        let _ = tx.send(Err(anyhow::anyhow!("OpenAI-compatible endpoint error: {txt}"))).await;
        return;
    }

    let mut stream = res.bytes_stream();
    let mut carry  = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(b)  => b,
            Err(e) => { let _ = tx.send(Err(anyhow::anyhow!(e.to_string()))).await; return; }
        };

        carry.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = carry.find('\n') {
            let line = carry[..pos].trim_end_matches('\r').to_string();
            carry.drain(..=pos);
            if line.is_empty() { continue; }

            let payload = match line.strip_prefix("data:") {
                Some(p) => p.trim(),
                None    => continue,
            };
            if payload == "[DONE]" { return; }

            let v: Value = match serde_json::from_str(payload) {
                Ok(v)  => v,
                Err(_) => continue,
            };

            if let Some(t) = v.pointer("/choices/0/delta/content").and_then(|x| x.as_str()) {
                if tx.send(Ok(t.to_string())).await.is_err() {
                    return;
                }
            }
        }
    }
}
