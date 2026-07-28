//! LLM provider (BYOK) query resolvers — add these to your existing QueryRoot.

use async_graphql::*;
use uuid::Uuid;
use crate::server::AppState;
use fluvio_database::db::llm_providers;
use super::llm_providers_type::GqlLlmProvider;

/// Get all LLM provider connections for the authenticated user.
/// `group_id = None` → personal scope; `Some(id)` → that company-brain group.
pub async fn get_user_llm_providers(
    ctx:      &Context<'_>,
    group_id: Option<String>,
) -> Result<Vec<GqlLlmProvider>> {
    let state   = ctx.data::<AppState>()?;
    let user_id = extract_user_id(ctx)?;

    let group_id = group_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| Error::new("invalid group_id"))?;

    Ok(llm_providers::get_user_providers(&state.pool, user_id, group_id)
        .await
        .map_err(|e| Error::new(e.to_string()))?
        .into_iter()
        .map(GqlLlmProvider::from)
        .collect())
}

fn extract_user_id(ctx: &Context<'_>) -> Result<Uuid> {
    ctx.data::<Uuid>()
        .map(|u| *u)
        .map_err(|_| Error::new("x-user-id header missing"))
}
