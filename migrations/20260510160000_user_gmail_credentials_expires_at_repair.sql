-- Fix older DBs where user_gmail_credentials had no expires_at (or column drift).
ALTER TABLE user_gmail_credentials
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ;

UPDATE user_gmail_credentials
SET expires_at = COALESCE(updated_at, now())
WHERE expires_at IS NULL;

ALTER TABLE user_gmail_credentials
    ALTER COLUMN expires_at SET NOT NULL;

CREATE INDEX IF NOT EXISTS user_gmail_credentials_expires_idx ON user_gmail_credentials(expires_at);
