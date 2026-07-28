//! Native Google Gemini `generateContent` API client.

use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Context;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::types::{ProviderConfig, Message};

fn require_key(cfg: &ProviderConfig) -> anyhow::Result<&str> {
    cfg.api_key.as_deref()
        .filter(|k| !k.trim().is_empty())
        .context("Gemini connection is missing an API key")
}

fn base_url(cfg: &ProviderConfig) -> String {
    cfg.base_url.clone()
        .unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string())
}

/// Gemini has no separate "system" role — it's a top-level `systemInstruction`,
/// and message roles are "user"/"model" rather than "user"/"assistant".
fn to_gemini_role(role: &str) -> &str {
    if role == "assistant" { "model" } else { "user" }
}

fn build_contents(messages: &[Message]) -> Vec<Value> {
    messages.iter()
        .filter(|m| !m.content.trim().is_empty())
        .map(|m| json!({
            "role":  to_gemini_role(&m.role),
            "parts": [{ "text": m.content }],
        }))
        .collect()
}

pub async fn chat(
    cfg:      &ProviderConfig,
    system:   &str,
    messages: &[Message],
) -> anyhow::Result<String> {
    let api_key = require_key(cfg)?;
    let client  = Client::new();
    let model   = cfg.model();
    let url = format!("{}/v1beta/models/{model}:generateContent", base_url(cfg));

    let resp: Value = client
        .post(&url)
        .header("x-goog-api-key", api_key)
        .header("content-type",   "application/json")
        .json(&json!({
            "systemInstruction": { "parts": [{ "text": system }] },
            "contents":          build_contents(messages),
        }))
        .send()
        .await
        .context("failed to reach Gemini API")?
        .json()
        .await
        .context("failed to parse Gemini response")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("Gemini error: {err}");
    }

    let answer = resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .context("no text in Gemini response")?
        .to_string();

    Ok(answer)
}

pub async fn chat_streaming(
    cfg:      ProviderConfig,
    system:   String,
    messages: Vec<Message>,
    tx:       mpsc::Sender<anyhow::Result<String>>,
) {
    let api_key = match require_key(&cfg) {
        Ok(k)  => k.to_string(),
        Err(e) => { let _ = tx.send(Err(e)).await; return; }
    };
    let client = Client::new();
    let model  = cfg.model();
    let url = format!("{}/v1beta/models/{model}:streamGenerateContent?alt=sse", base_url(&cfg));

    let res = match client
        .post(&url)
        .header("x-goog-api-key", &api_key)
        .header("content-type",   "application/json")
        .json(&json!({
            "systemInstruction": { "parts": [{ "text": system }] },
            "contents":          build_contents(&messages),
        }))
        .send()
        .await
    {
        Ok(r)  => r,
        Err(e) => { let _ = tx.send(Err(anyhow::anyhow!(e.to_string()))).await; return; }
    };

    if !res.status().is_success() {
        let txt = res.text().await.unwrap_or_default();
        let _ = tx.send(Err(anyhow::anyhow!("Gemini error: {txt}"))).await;
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

            let v: Value = match serde_json::from_str(payload) {
                Ok(v)  => v,
                Err(_) => continue,
            };

            if let Some(t) = v.pointer("/candidates/0/content/parts/0/text").and_then(|x| x.as_str()) {
                if tx.send(Ok(t.to_string())).await.is_err() {
                    return;
                }
            }
        }
    }
}
