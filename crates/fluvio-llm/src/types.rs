//! Provider-agnostic types shared by every wire-format implementation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Anthropic,
    // `rename_all = "snake_case"` would otherwise produce "open_ai" (word
    // boundary before the capital A) — must match `as_str()`/`parse()` and
    // the Postgres `llm_provider_kind` enum, which all use "openai".
    #[serde(rename = "openai")]
    OpenAi,
    Gemini,
    Ollama,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::OpenAi    => "openai",
            Provider::Gemini    => "gemini",
            Provider::Ollama    => "ollama",
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "anthropic" => Ok(Provider::Anthropic),
            "openai"    => Ok(Provider::OpenAi),
            "gemini"    => Ok(Provider::Gemini),
            "ollama"    => Ok(Provider::Ollama),
            other       => Err(anyhow::anyhow!("unknown LLM provider: {other}")),
        }
    }

    /// Compiled-in default model when a connection doesn't override one.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::OpenAi    => "gpt-4o",
            Provider::Gemini    => "gemini-2.0-flash",
            Provider::Ollama    => "llama3.1",
        }
    }
}

/// Everything needed to make one chat call: which provider, credentials, and
/// (optionally) an endpoint/model override. Resolved per-request from either
/// a user's BYOK connection or a deployment-level fallback — never held
/// long-term.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: Provider,
    /// `None` only valid for `Provider::Ollama`.
    pub api_key:  Option<String>,
    /// Required for `Ollama`; optional custom endpoint override otherwise.
    pub base_url: Option<String>,
    /// `None` → use `provider.default_model()`.
    pub model:    Option<String>,
}

impl ProviderConfig {
    pub fn model(&self) -> &str {
        self.model.as_deref().unwrap_or_else(|| self.provider.default_model())
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role:    String,
    pub content: String,
}
