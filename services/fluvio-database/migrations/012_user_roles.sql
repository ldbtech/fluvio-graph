-- 012_user_roles.sql
-- Add role and must_change_password columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'member';
ALTER TABLE users ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE;

-- Automatically promote existing company creators to admin
UPDATE users 
SET role = 'admin' 
WHERE id IN (SELECT created_by FROM companies);
