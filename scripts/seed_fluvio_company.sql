-- Seed script for fluvio_company database
-- Target company: 5f7258bc-4a09-45de-87e1-05ec40573408 (Vowayage)
-- Target user: 7eceeae5-a8ef-4d61-9e50-c99a955dbd11 (Alice Owner)

-- Clean up existing data first
TRUNCATE execution_logs, action_authorizations, document_reconciliations, pipeline_runs RESTART IDENTITY;

-- 1. Seed Document Reconciliations
INSERT INTO document_reconciliations (company_id, title, description, source_a, source_b, resolved_to, time_ago)
VALUES 
('5f7258bc-4a09-45de-87e1-05ec40573408', 
 'User Retention Cohort Conflict', 
 'Notion analytics specification defined ''active user'' as logged in within 30 days, but Slack channel #data-science conversation between Ali and Sarah agreed on 14 days.',
 'Notion (Analytics Spec v2.1)', 
 'Slack (#data-science)', 
 '14 days (Reconciled with active SQL query definition in production database)', 
 '1h ago'),
('5f7258bc-4a09-45de-87e1-05ec40573408', 
 'Data Ingest Window Conflict', 
 'Notion ingestion documentation specified a 90-day retention sync window, but python churn script cron configuration is hardcoded to 60 days.',
 'Notion (Ingestion Docs)', 
 'GitHub (churn_train.py)', 
 '60 days (Resolved by configuration code active environment settings)', 
 '4h ago'),
('5f7258bc-4a09-45de-87e1-05ec40573408', 
 'Monthly CAC Calculation Formula', 
 'Notion total marketing ad spend report omitted the PR agency retainer fee, but a Slack conversation specified adding $12,000 monthly PR additions to Customer Acquisition Cost.',
 'Notion (Ad Spend Report)', 
 'Slack (#marketing-telemetry)', 
 '$12,000 additions included (Validated via live Tableau calculated field state)', 
 '1d ago');

-- 2. Seed Action Authorizations (HITL Reviews)
INSERT INTO action_authorizations (company_id, action_type, description, severity, initiated_by_user_id, status)
VALUES
('5f7258bc-4a09-45de-87e1-05ec40573408', 
 'deploy_dashboard', 
 'Deploy new Tableau dashboard for Executive Q2 Customer Acquisition Cost (CAC) review', 
 'high', 
 '7eceeae5-a8ef-4d61-9e50-c99a955dbd11', 
 'pending'),
('5f7258bc-4a09-45de-87e1-05ec40573408', 
 'train_model', 
 'Trigger re-training run for Customer Churn Prediction Model (v3.2)', 
 'medium', 
 '7eceeae5-a8ef-4d61-9e50-c99a955dbd11', 
 'pending');

-- 3. Seed Pipeline Runs
INSERT INTO pipeline_runs (company_id, name, agent_name, status, progress, detail)
VALUES
('5f7258bc-4a09-45de-87e1-05ec40573408', 'Data Pipeline Ingestion', 'Data Engineer', 'syncing', 65, 'Syncing Notion handbooks, Slack transcripts...'),
('5f7258bc-4a09-45de-87e1-05ec40573408', 'Knowledge Syncer', 'Ingestion Bot', 'completed', 100, 'Completed indexing for engineering guidelines');

-- 4. Seed Execution Logs
INSERT INTO execution_logs (company_id, initiated_by_user_id, agent_name, message, log_level)
VALUES
('5f7258bc-4a09-45de-87e1-05ec40573408', '7eceeae5-a8ef-4d61-9e50-c99a955dbd11', 'Data Engineer Agent', 'Initiating data ingestion pipeline sync from Notion', 'info'),
('5f7258bc-4a09-45de-87e1-05ec40573408', '7eceeae5-a8ef-4d61-9e50-c99a955dbd11', 'BI & Dashboard Agent', 'Dashboard schema compilation completed with 0 errors', 'success'),
('5f7258bc-4a09-45de-87e1-05ec40573408', '7eceeae5-a8ef-4d61-9e50-c99a955dbd11', 'Security Auditor', 'Detected high-risk transaction in action_authorizations: deploy_dashboard', 'warning');
