//! LLM provider (BYOK) mutation resolvers — add these to your existing MutationRoot.

use async_graphql::*;
use uuid::Uuid;

use crate::server::AppState;
use fluvio_database::db::llm_providers;
use fluvio_llm::types::Provider;
use super::llm_providers_type::*;

/// Connect (or reconnect) an LLM provider for the authenticated user.
pub async fn connect_llm_provider(
    ctx:   &Context<'_>,
    input: ConnectLlmProviderInput,
) -> Result<GqlLlmProvider> {
    let state   = ctx.data::<AppState>()?;
    let user_id = extract_user_id(ctx)?;

    let provider = Provider::parse(&input.provider)
        .map_err(|e| Error::new(e.to_string()))?;

    let group_id = input.group_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| Error::new("invalid group_id"))?;

    if provider == Provider::Ollama {
        if input.base_url.as_deref().unwrap_or("").trim().is_empty() {
            return Err(Error::new("base_url is required for ollama"));
        }
    } else if input.api_key.as_deref().unwrap_or("").trim().is_empty() {
        return Err(Error::new(format!("api_key is required for {}", provider.as_str())));
    }

    let ciphertext = match &input.api_key {
        Some(key) if !key.trim().is_empty() => {
            let credential_key = state.credential_key.as_ref().ok_or_else(|| {
                Error::new(
                    "LLM credential encryption is not configured on this deployment — \
                     set FLUVIOME_CREDENTIAL_KEY",
                )
            })?;
            Some(fluvio_llm::crypto::encrypt(credential_key, key)
                .map_err(|e| Error::new(e.to_string()))?)
        }
        _ => None,
    };

    let saved = llm_providers::upsert(
        &state.pool,
        user_id,
        group_id,
        provider.as_str(),
        ciphertext.as_deref(),
        input.base_url.as_deref(),
        input.default_model.as_deref(),
    ).await.map_err(|e| Error::new(e.to_string()))?;

    tracing::info!(user_id = %user_id, provider = %provider.as_str(), "llm provider connected");

    Ok(GqlLlmProvider::from(saved))
}

/// Disconnect an LLM provider connection. Only the owning user may do this.
pub async fn disconnect_llm_provider(
    ctx: &Context<'_>,
    id:  String,
) -> Result<bool> {
    let state   = ctx.data::<AppState>()?;
    let user_id = extract_user_id(ctx)?;
    let id      = Uuid::parse_str(&id).map_err(|_| Error::new("invalid id"))?;

    let existing = llm_providers::get_provider(&state.pool, id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("llm provider connection not found"))?;

    if existing.user_id != user_id {
        return Err(Error::new("permission denied"));
    }

    Ok(llm_providers::delete_llm_provider(&state.pool, id)
        .await
        .map_err(|e| Error::new(e.to_string()))?)
}

/// Mark a connection as the user's default for its scope. Only the owning
/// user may do this.
pub async fn set_default_llm_provider(
    ctx: &Context<'_>,
    id:  String,
) -> Result<GqlLlmProvider> {
    let state   = ctx.data::<AppState>()?;
    let user_id = extract_user_id(ctx)?;
    let id      = Uuid::parse_str(&id).map_err(|_| Error::new("invalid id"))?;

    let existing = llm_providers::get_provider(&state.pool, id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .ok_or_else(|| Error::new("llm provider connection not found"))?;

    if existing.user_id != user_id {
        return Err(Error::new("permission denied"));
    }

    Ok(GqlLlmProvider::from(
        llm_providers::set_default(&state.pool, id)
            .await
            .map_err(|e| Error::new(e.to_string()))?
    ))
}

fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing"))
}
