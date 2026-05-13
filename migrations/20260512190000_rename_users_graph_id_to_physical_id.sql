-- Align existing `users` tables with `physical_id` (NFC / card scope UUID).
-- Safe to run more than once.

-- 1) Legacy installs: column was `graph_id` → rename in place (keeps values + UNIQUE).
DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'users'
      AND column_name = 'graph_id'
  )
  AND NOT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'users'
      AND column_name = 'physical_id'
  ) THEN
    ALTER TABLE users RENAME COLUMN graph_id TO physical_id;
  END IF;
END $$;

-- 2) Very old / partial state: no scope column at all → add one.
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = 'users'
      AND column_name = 'physical_id'
  ) THEN
    ALTER TABLE users
      ADD COLUMN physical_id UUID UNIQUE DEFAULT uuid_generate_v4();
  END IF;
END $$;
