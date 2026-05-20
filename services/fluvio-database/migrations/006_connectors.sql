-- 006_connectors.sql
-- Stores OAuth/token credentials for external service connectors.
--
-- Scope:
--   group_id = NULL  → personal digital twin connector
--   group_id = UUID  → company brain group connector

CREATE TYPE connector_kind AS ENUM ('github', 'notion', 'zoom');
CREATE TYPE connector_status AS ENUM ('connected', 'syncing', 'error', 'disconnected');
CREATE TYPE auth_method AS ENUM ('token', 'oauth');

CREATE TABLE IF NOT EXISTS connectors (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id          UUID NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    group_id         UUID          REFERENCES groups(id) ON DELETE CASCADE,
    kind             connector_kind   NOT NULL,
    auth_method      auth_method      NOT NULL,
    access_token     TEXT NOT NULL,
    refresh_token    TEXT,
    token_expires_at TIMESTAMPTZ,
    status           connector_status NOT NULL DEFAULT 'connected',
    error_message    TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_sync_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_connectors_user_id  ON connectors (user_id);
CREATE INDEX IF NOT EXISTS idx_connectors_group_id ON connectors (group_id);
CREATE INDEX IF NOT EXISTS idx_connectors_kind     ON connectors (kind);
CREATE INDEX IF NOT EXISTS idx_connectors_status   ON connectors (status);

-- Unique: one connector per kind per user for personal twin (group_id IS NULL)
CREATE UNIQUE INDEX IF NOT EXISTS idx_connectors_unique_personal
    ON connectors (user_id, kind)
    WHERE group_id IS NULL;

-- Unique: one connector per kind per user per group for company brain
CREATE UNIQUE INDEX IF NOT EXISTS idx_connectors_unique_group
    ON connectors (user_id, kind, group_id)
    WHERE group_id IS NOT NULL;