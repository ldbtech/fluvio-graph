-- 002_groups.sql
-- A group = one collaborative knowledge graph.
-- graph_id is the SurrealDB namespace key for this group's graph.

CREATE TABLE IF NOT EXISTS groups (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    description TEXT,
    graph_id    UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    created_by  UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_groups_created_by ON groups (created_by);
CREATE INDEX IF NOT EXISTS idx_groups_graph_id   ON groups (graph_id);