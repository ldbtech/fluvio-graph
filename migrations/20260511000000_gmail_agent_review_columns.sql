-- Store AI reply text and thread id for dashboard "review drafts" UX.
ALTER TABLE user_gmail_agent_processed
    ADD COLUMN IF NOT EXISTS reply_proposal TEXT,
    ADD COLUMN IF NOT EXISTS thread_id TEXT,
    ADD COLUMN IF NOT EXISTS subject_hint TEXT;
