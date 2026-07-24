-- sqlx:no-transaction
ALTER TYPE connector_kind ADD VALUE IF NOT EXISTS 'postgresql';
ALTER TYPE connector_kind ADD VALUE IF NOT EXISTS 'mysql';
ALTER TYPE connector_kind ADD VALUE IF NOT EXISTS 'mongodb';
ALTER TYPE connector_kind ADD VALUE IF NOT EXISTS 'redis';
ALTER TYPE connector_kind ADD VALUE IF NOT EXISTS 'snowflake';
ALTER TYPE connector_kind ADD VALUE IF NOT EXISTS 'bigquery';

ALTER TYPE resource_kind ADD VALUE IF NOT EXISTS 'database_table';
