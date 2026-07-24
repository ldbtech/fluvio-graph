-- 016_planner_team_approvals.sql
-- Add team association to workspaces and create planner approvals table

-- 1. Add team_id to workspaces
ALTER TABLE workspaces ADD COLUMN IF NOT EXISTS team_id UUID REFERENCES teams(id) ON DELETE SET NULL;

-- 2. Create planner approval status type if not exists
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'planner_approval_status') THEN
        CREATE TYPE planner_approval_status AS ENUM ('pending', 'approved', 'rejected');
    END IF;
END$$;

-- 3. Create planner approvals table
CREATE TABLE IF NOT EXISTS planner_approvals (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id     UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    suggested_by     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    change_type      TEXT NOT NULL,
    change_details   JSONB NOT NULL DEFAULT '{}',
    status           planner_approval_status NOT NULL DEFAULT 'pending',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at      TIMESTAMPTZ,
    review_note      TEXT
);

-- 4. Create Indexes
CREATE INDEX IF NOT EXISTS idx_planner_approvals_workspace ON planner_approvals (workspace_id);
CREATE INDEX IF NOT EXISTS idx_planner_approvals_status ON planner_approvals (workspace_id, status);
