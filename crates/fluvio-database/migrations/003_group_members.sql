-- 003_group_members.sql
-- Membership table — user ↔ group with role.
-- A user can be in multiple groups.
-- A group can have multiple owners.

CREATE TYPE member_role AS ENUM ('owner', 'contributor', 'trusted', 'viewer');

CREATE TABLE IF NOT EXISTS group_members (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id    UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users(id)  ON DELETE CASCADE,
    role        member_role NOT NULL DEFAULT 'contributor',
    invited_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (group_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_members_group_id ON group_members (group_id);
CREATE INDEX IF NOT EXISTS idx_members_user_id  ON group_members (user_id);
CREATE INDEX IF NOT EXISTS idx_members_role     ON group_members (group_id, role);