-- Gmail OAuth tokens per Fluvio user (replaces ~/.fluvio/credentials/gmail.json for logged-in flows).

CREATE TABLE user_gmail_credentials (
    user_id       UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    access_token  TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX user_gmail_credentials_expires_idx ON user_gmail_credentials(expires_at);
