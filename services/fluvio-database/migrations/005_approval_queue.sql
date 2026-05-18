-- 005_approval_queue.sql
-- Tracks contributions awaiting owner review.
-- surreal_node_id links back to the pending node in SurrealDB-collab.

CREATE TYPE contribution_kind AS ENUM (
    'knowledge',   -- text/PDF knowledge node
    'tool',        -- tool definition
    'agent',       -- agent definition
    'connector',   -- connector configuration
    'pdf'          -- PDF upload (multiple knowledge nodes)
);

CREATE TYPE approval_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE IF NOT EXISTS approval_queue (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id         UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    contributed_by   UUID NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    kind             contribution_kind NOT NULL,
    surreal_node_id  TEXT NOT NULL,      -- node id in SurrealDB-collab
    status           approval_status NOT NULL DEFAULT 'pending',
    reviewed_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    review_note      TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_queue_group_id      ON approval_queue (group_id);
CREATE INDEX IF NOT EXISTS idx_queue_status        ON approval_queue (group_id, status);
CREATE INDEX IF NOT EXISTS idx_queue_contributed_by ON approval_queue (contributed_by);