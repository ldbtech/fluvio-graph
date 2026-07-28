//! HTTP client for database-server's internal (non-GraphQL, non-public)
//! `/internal/resolve-llm-credential` route. Used by twin-server and
//! collab-server to resolve a per-user BYOK provider connection (or the
//! deployment-level fallback) before calling an LLM.
//!
//! Deliberately NOT GraphQL: this route returns a decrypted secret, and this
//! codebase's Apollo supergraph is statically pre-built (`rover supergraph
//! compose`, baked into the gateway image) with no `@inaccessible`/entity
//! federation used anywhere — a plain axum route outside the async-graphql
//! schema is the only way to guarantee this can never end up in the public
//! supergraph regardless of when/how composition is re-run.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{Provider, ProviderConfig};

#[derive(Serialize)]
struct ResolveRequest {
    user_id:  Uuid,
    group_id: Option<Uuid>,
    provider: Option<Provider>,
}

#[derive(Deserialize)]
struct ResolveResponse {
    provider: Provider,
    api_key:  Option<String>,
    base_url: Option<String>,
    model:    Option<String>,
}

#[derive(Clone)]
pub struct CredentialResolver {
    base_url:        String,
    internal_secret: Option<String>,
    client:          reqwest::Client,
}

impl CredentialResolver {
    /// `base_url` is the bare service base (e.g. `http://fluvio-database:3005`),
    /// NOT a `/graphql`-suffixed URL — this hits a plain HTTP route.
    pub fn new(base_url: impl Into<String>, internal_secret: Option<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            internal_secret,
            client: reqwest::Client::new(),
        }
    }

    /// Resolves the caller's active LLM connection. `provider = None` resolves
    /// the user's default connection (falling back to the deployment default
    /// for whichever provider that fallback is configured for).
    pub async fn resolve(
        &self,
        user_id:  Uuid,
        group_id: Option<Uuid>,
        provider: Option<Provider>,
    ) -> anyhow::Result<ProviderConfig> {
        let url = format!("{}/internal/resolve-llm-credential", self.base_url);

        let mut req = self.client.post(&url).json(&ResolveRequest { user_id, group_id, provider });
        if let Some(secret) = &self.internal_secret {
            req = req.header("x-internal-auth", secret);
        }

        let resp = req.send().await
            .map_err(|e| anyhow::anyhow!("failed to reach fluvio-database credential resolver: {e}"))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("no LLM provider configured — connect one, or set a deployment-level fallback key");
        }
        if !resp.status().is_success() {
            let txt = resp.text().await.unwrap_or_default();
            anyhow::bail!("credential resolution failed: {txt}");
        }

        let parsed: ResolveResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("failed to parse credential resolution response: {e}"))?;

        Ok(ProviderConfig {
            provider: parsed.provider,
            api_key:  parsed.api_key,
            base_url: parsed.base_url,
            model:    parsed.model,
        })
    }
}
