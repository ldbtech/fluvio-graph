-- 014_user_twin_roles.sql
-- Add assigned_agent_roles text array column to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS assigned_agent_roles TEXT[] NOT NULL DEFAULT '{}';

-- Pre-fill company owners with all agent roles as they manage everything
UPDATE users 
SET assigned_agent_roles = '{"CustomerSuccessAgentRole", "DataPipelineAgentRole", "AuditorAgentRole"}' 
WHERE id IN (SELECT created_by FROM companies);
