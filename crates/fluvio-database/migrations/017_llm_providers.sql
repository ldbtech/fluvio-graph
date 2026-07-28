-- 017_llm_providers.sql
-- Per-user (optionally per-group/company-brain) LLM provider connections (BYOK).
--
-- Scope mirrors connectors (006_connectors.sql):
--   group_id = NULL  → personal digital twin scope
--   group_id = UUID  → company brain group scope

CREATE TYPE llm_provider_kind AS ENUM ('anthropic', 'openai', 'gemini', 'ollama');

CREATE TABLE IF NOT EXISTS llm_providers (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id            UUID NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    group_id           UUID          REFERENCES groups(id) ON DELETE CASCADE,
    provider           llm_provider_kind NOT NULL,
    api_key_ciphertext BYTEA,          -- NULL only allowed for 'ollama'
    base_url           TEXT,           -- required for 'ollama', optional custom endpoint otherwise
    default_model      TEXT,
    is_default         BOOLEAN NOT NULL DEFAULT false,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT llm_providers_key_required
        CHECK (provider = 'ollama' OR api_key_ciphertext IS NOT NULL),
    CONSTRAINT llm_providers_ollama_base_url_required
        CHECK (provider <> 'ollama' OR base_url IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_llm_providers_user_id  ON llm_providers (user_id);
CREATE INDEX IF NOT EXISTS idx_llm_providers_group_id ON llm_providers (group_id);

-- One connection per provider per user for personal scope
CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_providers_unique_personal
    ON llm_providers (user_id, provider)
    WHERE group_id IS NULL;

-- One connection per provider per user per group for company brain
CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_providers_unique_group
    ON llm_providers (user_id, provider, group_id)
    WHERE group_id IS NOT NULL;

-- At most one default connection per scope
CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_providers_one_default_personal
    ON llm_providers (user_id)
    WHERE is_default AND group_id IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_providers_one_default_group
    ON llm_providers (user_id, group_id)
    WHERE is_default AND group_id IS NOT NULL;
