//! Single dispatch surface — every caller (twin-server, collab-server,
//! database-server, and their Python equivalent) goes through `chat`/
//! `chat_streaming` rather than talking to a provider's wire format directly.

use tokio::sync::mpsc;

use crate::types::{Provider, ProviderConfig, Message};
use crate::{anthropic, gemini, openai_compat};

pub async fn chat(
    cfg:      &ProviderConfig,
    system:   &str,
    messages: &[Message],
) -> anyhow::Result<String> {
    match cfg.provider {
        Provider::Anthropic => anthropic::chat(cfg, system, messages).await,
        Provider::Gemini    => gemini::chat(cfg, system, messages).await,
        Provider::OpenAi | Provider::Ollama => openai_compat::chat(cfg, system, messages).await,
    }
}

/// Streaming chat — sends chunks via mpsc channel as they arrive.
pub async fn chat_streaming(
    cfg:      ProviderConfig,
    system:   String,
    messages: Vec<Message>,
    tx:       mpsc::Sender<anyhow::Result<String>>,
) {
    match cfg.provider {
        Provider::Anthropic => anthropic::chat_streaming(cfg, system, messages, tx).await,
        Provider::Gemini    => gemini::chat_streaming(cfg, system, messages, tx).await,
        Provider::OpenAi | Provider::Ollama => openai_compat::chat_streaming(cfg, system, messages, tx).await,
    }
}
