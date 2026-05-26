-- Table definitions for the new fluvio_company database

-- 1. Execution Logs Table
CREATE TABLE IF NOT EXISTS execution_logs (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id             UUID NOT NULL,
    initiated_by_user_id   UUID NOT NULL,
    initiated_by_twin_id   UUID,
    agent_name             TEXT NOT NULL,
    message                TEXT NOT NULL,
    log_level              TEXT NOT NULL DEFAULT 'info',
    timestamp              TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. Action Authorizations Table
CREATE TABLE IF NOT EXISTS action_authorizations (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id             UUID NOT NULL,
    action_type            TEXT NOT NULL,
    description            TEXT NOT NULL,
    severity               TEXT NOT NULL DEFAULT 'medium',
    initiated_by_user_id   UUID NOT NULL,
    status                 TEXT NOT NULL DEFAULT 'pending',
    authorized_by_user_id  UUID,
    notes                  TEXT,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at            TIMESTAMPTZ
);

-- 3. Document Reconciliations Table
CREATE TABLE IF NOT EXISTS document_reconciliations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    source_a        TEXT NOT NULL,
    source_b        TEXT NOT NULL,
    resolved_to     TEXT NOT NULL,
    time_ago        TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 4. Pipeline Runs Table
CREATE TABLE IF NOT EXISTS pipeline_runs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id      UUID NOT NULL,
    name            TEXT NOT NULL,
    agent_name      TEXT NOT NULL,
    status          TEXT NOT NULL,
    progress        INTEGER NOT NULL DEFAULT 0,
    detail          TEXT,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_exec_logs_company ON execution_logs(company_id);
CREATE INDEX IF NOT EXISTS idx_exec_logs_user ON execution_logs(initiated_by_user_id);
CREATE INDEX IF NOT EXISTS idx_actions_company ON action_authorizations(company_id);
CREATE INDEX IF NOT EXISTS idx_actions_status ON action_authorizations(status);
CREATE INDEX IF NOT EXISTS idx_reconcile_company ON document_reconciliations(company_id);
CREATE INDEX IF NOT EXISTS idx_pipelines_company ON pipeline_runs(company_id);
