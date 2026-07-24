-- 011_planner_chat_messages.sql
-- Stores conversation history between the user and the AI Architect per workspace.

CREATE TABLE IF NOT EXISTS planner_chat_messages (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id  UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    sender        TEXT NOT NULL, -- 'user' or 'ai'
    content       TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_planner_chat_workspace ON planner_chat_messages (workspace_id);
CREATE INDEX IF NOT EXISTS idx_planner_chat_created_at ON planner_chat_messages (workspace_id, created_at ASC);
