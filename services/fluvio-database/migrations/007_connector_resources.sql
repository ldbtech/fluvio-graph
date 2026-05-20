-- 007_connector_repos.sql
-- Tracks repositories/resources available for each connector.
--
-- GitHub: one row per repo the user has access to
-- Notion:  one row per page/database the integration has access to
-- Zoom:    one row per meeting recording
--
-- selected = true means the user wants this resource synced.
-- node_count tracks how many graph nodes were created from this resource.
-- last_sync_at tracks when this specific resource was last synced.

CREATE TYPE resource_kind AS ENUM (
    'github_repo',      -- GitHub repository
    'notion_page',      -- Notion page
    'notion_database',  -- Notion database
    'zoom_recording'    -- Zoom cloud recording
);

CREATE TABLE IF NOT EXISTS connector_resources (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connector_id     UUID NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    resource_kind    resource_kind NOT NULL,

    -- External identifier — stable across syncs
    -- GitHub:  "owner/repo-name"  e.g. "ali/kg-engine"
    -- Notion:  page UUID from Notion API
    -- Zoom:    meeting UUID from Zoom API
    external_id      TEXT NOT NULL,

    -- Human-readable name shown in UI
    name             TEXT NOT NULL,
    description      TEXT,

    -- User explicitly selected this for sync
    selected         BOOLEAN NOT NULL DEFAULT false,

    -- Sync tracking
    last_sync_at     TIMESTAMPTZ,
    node_count       INT NOT NULL DEFAULT 0,

    -- Extra metadata (JSON) — repo language, page icon, etc.
    meta             JSONB NOT NULL DEFAULT '{}',

    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (connector_id, external_id)
);

CREATE INDEX IF NOT EXISTS idx_resources_connector_id ON connector_resources (connector_id);
CREATE INDEX IF NOT EXISTS idx_resources_selected     ON connector_resources (connector_id, selected);
CREATE INDEX IF NOT EXISTS idx_resources_external_id  ON connector_resources (external_id);