//! Anthropic API client.
//!
//! Supports both streaming (SSE) and non-streaming responses.
//! Ported from the monolith's twin chat handler.

use reqwest::Client;
use serde_json::{json, Value};
use anyhow::Context;
use futures_util::StreamExt;
use tokio::sync::mpsc;

pub const MODEL: &str = "claude-sonnet-4-20250514";
pub const MAX_TOKENS: u32 = 4096;

#[derive(Debug, Clone)]
pub struct Message {
    pub role:    String,
    pub content: String,
}

/// Non-streaming chat — returns the full answer at once.
pub async fn chat(
    api_key:  &str,
    system:   &str,
    messages: &[Message],
) -> anyhow::Result<String> {
    let client = Client::new();

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
            "model":      MODEL,
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

    let answer = resp["content"][0]["text"]
        .as_str()
        .context("no text in Anthropic response")?
        .to_string();

    Ok(answer)
}

/// Streaming chat — sends chunks via mpsc channel as they arrive.
/// The receiver end is converted to an HTTP stream in the GraphQL resolver.
pub async fn chat_streaming(
    api_key:  String,
    system:   String,
    messages: Vec<Message>,
    tx:       mpsc::Sender<anyhow::Result<String>>,
) {
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
            "model":      MODEL,
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