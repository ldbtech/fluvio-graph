-- 015_user_twin_manifest.sql
-- Add twin_manifest text column to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS twin_manifest TEXT;
