-- Per-user Gmail inbox focus (sender allow-list) and History API cursor for incremental deltas.

CREATE TABLE user_gmail_focus_senders (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sender TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, sender)
);

CREATE INDEX idx_user_gmail_focus_senders_user ON user_gmail_focus_senders(user_id);

CREATE TABLE user_gmail_history_cursor (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    history_id TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
