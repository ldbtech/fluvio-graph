//! LLM provider connection (BYOK) table queries — pure data access.
//! Ciphertext in, ciphertext out — encryption/decryption happens in the caller
//! (database-server), which is the only place that holds the master key.

use sqlx::PgPool;
use uuid::Uuid;

use crate::db::queries::llm_providers::{LlmProvider,
    UPSERT_PERSONAL, UPSERT_GROUP, GET_BY_ID, GET_USER_PROVIDERS,
    GET_DEFAULT_FOR_USER, GET_FOR_USER_AND_PROVIDER,
    CLEAR_DEFAULT_PERSONAL, CLEAR_DEFAULT_GROUP, SET_DEFAULT, DELETE};

#[allow(clippy::too_many_arguments)]
pub async fn upsert(
    pool:               &PgPool,
    user_id:            Uuid,
    group_id:           Option<Uuid>,
    provider:           &str,
    api_key_ciphertext: Option<&[u8]>,
    base_url:           Option<&str>,
    default_model:      Option<&str>,
) -> anyhow::Result<LlmProvider> {
    Ok(match group_id {
        None => sqlx::query_as::<_, LlmProvider>(UPSERT_PERSONAL)
            .bind(user_id)
            .bind(provider)
            .bind(api_key_ciphertext)
            .bind(base_url)
            .bind(default_model)
            .fetch_one(pool)
            .await?,
        Some(group_id) => sqlx::query_as::<_, LlmProvider>(UPSERT_GROUP)
            .bind(user_id)
            .bind(group_id)
            .bind(provider)
            .bind(api_key_ciphertext)
            .bind(base_url)
            .bind(default_model)
            .fetch_one(pool)
            .await?,
    })
}

pub async fn get_provider(
    pool: &PgPool,
    id:   Uuid,
) -> anyhow::Result<Option<LlmProvider>> {
    Ok(sqlx::query_as::<_, LlmProvider>(GET_BY_ID)
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_user_providers(
    pool:     &PgPool,
    user_id:  Uuid,
    group_id: Option<Uuid>,
) -> anyhow::Result<Vec<LlmProvider>> {
    Ok(sqlx::query_as::<_, LlmProvider>(GET_USER_PROVIDERS)
        .bind(user_id)
        .bind(group_id)
        .fetch_all(pool)
        .await?)
}

pub async fn get_default_for_user(
    pool:     &PgPool,
    user_id:  Uuid,
    group_id: Option<Uuid>,
) -> anyhow::Result<Option<LlmProvider>> {
    Ok(sqlx::query_as::<_, LlmProvider>(GET_DEFAULT_FOR_USER)
        .bind(user_id)
        .bind(group_id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_for_user_and_provider(
    pool:     &PgPool,
    user_id:  Uuid,
    group_id: Option<Uuid>,
    provider: &str,
) -> anyhow::Result<Option<LlmProvider>> {
    Ok(sqlx::query_as::<_, LlmProvider>(GET_FOR_USER_AND_PROVIDER)
        .bind(user_id)
        .bind(group_id)
        .bind(provider)
        .fetch_optional(pool)
        .await?)
}

/// Sets `id` as the default for its scope, clearing any previous default first.
pub async fn set_default(
    pool: &PgPool,
    id:   Uuid,
) -> anyhow::Result<LlmProvider> {
    let target = get_provider(pool, id).await?
        .ok_or_else(|| anyhow::anyhow!("llm provider connection not found"))?;

    let mut tx = pool.begin().await?;

    match target.group_id {
        None => {
            sqlx::query(CLEAR_DEFAULT_PERSONAL).bind(target.user_id).execute(&mut *tx).await?;
        }
        Some(group_id) => {
            sqlx::query(CLEAR_DEFAULT_GROUP).bind(target.user_id).bind(group_id).execute(&mut *tx).await?;
        }
    }

    let updated = sqlx::query_as::<_, LlmProvider>(SET_DEFAULT)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(updated)
}

pub async fn delete_llm_provider(
    pool: &PgPool,
    id:   Uuid,
) -> anyhow::Result<bool> {
    let result = sqlx::query(DELETE)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
