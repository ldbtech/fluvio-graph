-- 004_invites.sql
-- Invite tokens — signed, single-use, expiring.
-- token is a UUID string embedded in an invite link.

CREATE TABLE IF NOT EXISTS invites (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    UUID NOT NULL REFERENCES groups(id)  ON DELETE CASCADE,
    invited_by  UUID NOT NULL REFERENCES users(id)   ON DELETE CASCADE,
    token       TEXT NOT NULL UNIQUE,
    role        member_role NOT NULL DEFAULT 'contributor',
    email       TEXT,                    -- optional target email
    expires_at  TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,             -- NULL = pending
    accepted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_invites_token    ON invites (token);
CREATE INDEX IF NOT EXISTS idx_invites_group_id ON invites (group_id);
CREATE INDEX IF NOT EXISTS idx_invites_email    ON invites (email);