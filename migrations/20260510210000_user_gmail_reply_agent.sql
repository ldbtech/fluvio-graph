-- Gmail reply agent: per-user prefs, idempotency, optional future outbox hints.

CREATE TABLE user_gmail_reply_agent (
    user_id          UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    enabled          BOOLEAN NOT NULL DEFAULT false,
    -- always_review | auto_when_confident
    send_mode        TEXT NOT NULL DEFAULT 'always_review'
        CHECK (send_mode IN ('always_review', 'auto_when_confident')),
    context_sources  JSONB NOT NULL DEFAULT '{}',
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_gmail_agent_processed (
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    gmail_message_id  TEXT NOT NULL,
    outcome           TEXT NOT NULL DEFAULT 'skipped'
        CHECK (outcome IN ('skipped', 'draft_only', 'sent', 'error')),
    detail            TEXT,
    processed_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, gmail_message_id)
);

CREATE INDEX idx_gmail_agent_processed_at ON user_gmail_agent_processed (user_id, processed_at DESC);
