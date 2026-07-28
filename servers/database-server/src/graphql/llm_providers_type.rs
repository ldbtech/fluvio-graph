//! GraphQL types for LLM provider (BYOK) connections.
//!
//! `GqlLlmProvider` deliberately has NO key/secret field — unlike the
//! pre-existing `GqlConnector.access_token` (a known plaintext exposure, out
//! of scope for this change), credentials never round-trip through GraphQL
//! at all. Decrypted-credential resolution is a separate, non-GraphQL,
//! internal-only route (`servers/database-server/src/internal.rs`).

use async_graphql::*;

use fluvio_database::db::queries::llm_providers::LlmProvider;

#[derive(SimpleObject, Clone)]
pub struct GqlLlmProvider {
    pub id:            String,
    pub user_id:       String,
    pub group_id:      Option<String>,
    pub provider:      String,
    pub base_url:      Option<String>,
    pub default_model: Option<String>,
    pub is_default:    bool,
    pub has_api_key:   bool,
    pub created_at:    String,
    pub updated_at:    String,
}

impl From<LlmProvider> for GqlLlmProvider {
    fn from(p: LlmProvider) -> Self {
        Self {
            id:            p.id.to_string(),
            user_id:       p.user_id.to_string(),
            group_id:      p.group_id.map(|g| g.to_string()),
            provider:      p.provider,
            base_url:      p.base_url,
            default_model: p.default_model,
            is_default:    p.is_default,
            has_api_key:   p.api_key_ciphertext.is_some(),
            created_at:    p.created_at.to_rfc3339(),
            updated_at:    p.updated_at.to_rfc3339(),
        }
    }
}

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(InputObject)]
pub struct ConnectLlmProviderInput {
    /// One of "anthropic", "openai", "gemini", "ollama".
    pub provider:      String,
    /// Required for anthropic/openai/gemini; optional for ollama.
    pub api_key:       Option<String>,
    /// Required for ollama; optional custom endpoint override otherwise.
    pub base_url:      Option<String>,
    pub default_model: Option<String>,
    pub group_id:      Option<String>,
}
