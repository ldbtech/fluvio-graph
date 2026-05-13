-- Background server poll: periodically run reply agent for opted-in users.
ALTER TABLE user_gmail_reply_agent
    ADD COLUMN IF NOT EXISTS auto_poll_enabled BOOLEAN NOT NULL DEFAULT false;
