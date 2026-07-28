//! Internal (non-GraphQL, non-public) LLM credential resolution.
//!
//! This is deliberately NOT part of the async-graphql schema — it hands back
//! a decrypted secret, and this codebase's Apollo supergraph is statically
//! pre-built (`rover supergraph compose`, baked into the gateway image at
//! build time) with no `@inaccessible`/entity-federation directive used
//! anywhere to selectively hide a schema field. A plain axum route outside
//! `build_schema` is the only way to guarantee this can never end up in the
//! public supergraph regardless of when/how composition is re-run.
//!
//! Every backend port is host-published in docker-compose today, so network
//! placement alone isn't real isolation — callers should set
//! `FLUVIOME_INTERNAL_SECRET` on any deployment reachable beyond localhost.

use axum::{http::HeaderMap, response::IntoResponse, routing::post, Json, Router};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use fluvio_database::db::llm_providers;
use fluvio_llm::types::{Provider, ProviderConfig};

use crate::server::AppState;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("no_provider_configured")]
    NotConfigured,
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Resolves the caller's active LLM connection: a specific provider if
/// requested, else the user's default; falls back to this deployment's
/// env-sourced key when the user has no connection at all.
pub async fn resolve_credential_inner(
    state:    &AppState,
    user_id:  Uuid,
    group_id: Option<Uuid>,
    provider: Option<Provider>,
) -> Result<ProviderConfig, ResolveError> {
    let row = match provider {
        Some(p) => llm_providers::get_for_user_and_provider(&state.pool, user_id, group_id, p.as_str()).await?,
        None    => llm_providers::get_default_for_user(&state.pool, user_id, group_id).await?,
    };

    if let Some(row) = row {
        let resolved_provider = Provider::parse(&row.provider)?;

        let api_key = match &row.api_key_ciphertext {
            Some(ciphertext) => {
                let key = state.credential_key.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "user has a connected LLM provider but this deployment has no \
                         FLUVIOME_CREDENTIAL_KEY configured to decrypt it"
                    )
                })?;
                Some(fluvio_llm::crypto::decrypt(key, ciphertext)?)
            }
            None => None,
        };

        return Ok(ProviderConfig {
            provider: resolved_provider,
            api_key,
            base_url: row.base_url,
            model:    row.default_model,
        });
    }

    fallback_provider_config(provider).ok_or(ResolveError::NotConfigured)
}

/// Deployment-level fallback, sourced from env vars read fresh on each call
/// (cheap, called only when a user has no BYOK connection). Keeps today's
/// single-key deployments working unchanged.
fn fallback_provider_config(requested: Option<Provider>) -> Option<ProviderConfig> {
    let try_provider = |p: Provider| -> Option<ProviderConfig> {
        match p {
            Provider::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok()
                .filter(|k| !k.trim().is_empty())
                .map(|k| ProviderConfig { provider: p, api_key: Some(k), base_url: None, model: None }),
            Provider::OpenAi => std::env::var("OPENAI_API_KEY").ok()
                .filter(|k| !k.trim().is_empty())
                .map(|k| ProviderConfig { provider: p, api_key: Some(k), base_url: None, model: None }),
            Provider::Gemini => std::env::var("GEMINI_API_KEY").ok()
                .filter(|k| !k.trim().is_empty())
                .map(|k| ProviderConfig { provider: p, api_key: Some(k), base_url: None, model: None }),
            Provider::Ollama => std::env::var("OLLAMA_BASE_URL").ok()
                .filter(|u| !u.trim().is_empty())
                .map(|u| ProviderConfig { provider: p, api_key: None, base_url: Some(u), model: None }),
        }
    };

    match requested {
        Some(p) => try_provider(p),
        // No specific provider requested — try in the order deployments have
        // historically configured a single key (ANTHROPIC_API_KEY first).
        None => [Provider::Anthropic, Provider::OpenAi, Provider::Gemini, Provider::Ollama]
            .into_iter()
            .find_map(try_provider),
    }
}

#[derive(Deserialize)]
struct ResolveRequest {
    user_id:  Uuid,
    group_id: Option<Uuid>,
    provider: Option<Provider>,
}

#[derive(Serialize)]
struct ResolveResponsePayload {
    provider: Provider,
    api_key:  Option<String>,
    base_url: Option<String>,
    model:    Option<String>,
}

async fn handler(state: AppState, headers: HeaderMap, req: ResolveRequest) -> axum::response::Response {
    if let Some(secret) = &state.internal_secret {
        let provided = headers.get("x-internal-auth").and_then(|v| v.to_str().ok());
        if provided != Some(secret.as_str()) {
            return (StatusCode::FORBIDDEN, "invalid or missing x-internal-auth").into_response();
        }
    }

    match resolve_credential_inner(&state, req.user_id, req.group_id, req.provider).await {
        Ok(cfg) => Json(ResolveResponsePayload {
            provider: cfg.provider,
            api_key:  cfg.api_key,
            base_url: cfg.base_url,
            model:    cfg.model,
        }).into_response(),
        Err(ResolveError::NotConfigured) => (StatusCode::NOT_FOUND, "no_provider_configured").into_response(),
        Err(ResolveError::Other(e))      => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// A `Router` carrying just the internal route, merged into the main app —
/// mirrors `graphql::graphql_router`'s shape.
pub fn router(state: AppState) -> Router {
    Router::new().route(
        "/internal/resolve-llm-credential",
        post(move |headers: HeaderMap, Json(req): Json<ResolveRequest>| {
            let state = state.clone();
            async move { handler(state, headers, req).await }
        }),
    )
}
