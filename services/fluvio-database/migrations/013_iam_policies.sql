-- 013_iam_policies.sql
-- Add policies text array column to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS policies TEXT[] NOT NULL DEFAULT '{"ReadOnlyAccess"}';

-- Pre-fill company owners with AdministratorAccess policy
UPDATE users 
SET policies = '{"AdministratorAccess"}' 
WHERE id IN (SELECT created_by FROM companies);
