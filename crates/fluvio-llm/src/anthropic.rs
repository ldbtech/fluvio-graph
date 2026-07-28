//! Native Anthropic Messages API client.
//! Ported from the original `fluvio-twin-core::llm::anthropic`, generalized
//! to take a `ProviderConfig` instead of a bare API key string.

use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Context;
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::types::{ProviderConfig, Message};

pub const MAX_TOKENS: u32 = 4096;

fn require_key(cfg: &ProviderConfig) -> anyhow::Result<&str> {
    cfg.api_key.as_deref()
        .filter(|k| !k.trim().is_empty())
        .context("Anthropic connection is missing an API key")
}

pub async fn chat(
    cfg:      &ProviderConfig,
    system:   &str,
    messages: &[Message],
) -> anyhow::Result<String> {
    let api_key = require_key(cfg)?;
    let client  = Client::new();

    let msgs: Vec<Value> = messages.iter()
        .filter(|m| !m.content.trim().is_empty())
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    let resp: Value = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key",         api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type",      "application/json")
        .json(&json!({
            "model":      cfg.model(),
            "max_tokens": MAX_TOKENS,
            "system":     system,
            "messages":   msgs,
        }))
        .send()
        .await
        .context("failed to reach Anthropic API")?
        .json()
        .await
        .context("failed to parse Anthropic response")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("Anthropic error: {err}");
    }

    let answer = resp["content"][0]["text"]
        .as_str()
        .context("no text in Anthropic response")?
        .to_string();

    Ok(answer)
}

/// Streaming chat — sends chunks via mpsc channel as they arrive.
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

    let msgs: Vec<Value> = messages.iter()
        .filter(|m| !m.content.trim().is_empty())
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect();

    let res = match client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key",         &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type",      "application/json")
        .json(&json!({
            "model":      cfg.model(),
            "max_tokens": MAX_TOKENS,
            "stream":     true,
            "system":     system,
            "messages":   msgs,
        }))
        .send()
        .await
    {
        Ok(r)  => r,
        Err(e) => {
            let _ = tx.send(Err(anyhow::anyhow!(e.to_string()))).await;
            return;
        }
    };

    if !res.status().is_success() {
        let txt = res.text().await.unwrap_or_default();
        let _ = tx.send(Err(anyhow::anyhow!("Anthropic error: {txt}"))).await;
        return;
    }

    let mut stream = res.bytes_stream();
    let mut carry  = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(b)  => b,
            Err(e) => {
                let _ = tx.send(Err(anyhow::anyhow!(e.to_string()))).await;
                return;
            }
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

            if v.get("type").and_then(|x| x.as_str()) == Some("content_block_delta") {
                if let Some(t) = v.pointer("/delta/text").and_then(|x| x.as_str()) {
                    if tx.send(Ok(t.to_string())).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}
