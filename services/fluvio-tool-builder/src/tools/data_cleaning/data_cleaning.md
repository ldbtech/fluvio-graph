# Data Cleaning Processor

## Purpose
Produce a trustworthy `clean_<table>` from a raw source table before any
downstream modeling, aggregation, or reporting. **You (the planner) author the
exact cleaning logic** against the real schema; this tool is a safe executor that
clones the source and runs your SQL against the clone. The raw source table is
never modified.

## Primary action: `run_cleaning`
Arguments:
- `table_name` (str, required) — the raw source table to clean.
- `statements` (list of SQL strings, required) — your cleaning steps, applied in
  order to the cloned table.
- `output_table` (str, optional) — defaults to `clean_<table_name>`.

The tool always:
1. Verifies the source table exists.
2. Clones it: `CREATE TABLE clean_<t> AS SELECT * FROM <t>`.
3. Runs each of your `statements` against the clone, in order.
4. Returns `{rows_processed, rows_remaining, rows_purged, statements_applied}`.

### Placeholders
In your statements use:
- `{table}` → the output/clean table (write here).
- `{source}` → the read-only source table (read only).

This keeps statements portable and makes intent explicit.

### Safety rail
Any statement that would mutate the source table (DROP/TRUNCATE/DELETE/UPDATE/
ALTER/INSERT targeting `{source}`) is rejected. Do all writes against `{table}`.

### Authoring guidance
Inspect the real columns from the workspace schema first, then author only the
transforms the data actually needs. Common building blocks (adapt to the schema —
do not paste blindly):

- **Normalize a header**: `ALTER TABLE {table} RENAME COLUMN "Email Address" TO email;`
- **Drop rows missing an identifier**: `DELETE FROM {table} WHERE email IS NULL OR email = '';`
- **Deduplicate**:
  ```sql
  CREATE TABLE {table}_dedup AS SELECT DISTINCT * FROM {table};
  DROP TABLE {table};
  ALTER TABLE {table}_dedup RENAME TO {table};
  ```
- **Trim / standardize text**: `UPDATE {table} SET country = upper(trim(country));`
- **Parse a currency string to numeric**:
  `UPDATE {table} SET revenue = regexp_replace(revenue, '[^0-9.]', '', 'g')::numeric WHERE revenue IS NOT NULL;`
- **Cast a type**: `ALTER TABLE {table} ALTER COLUMN signup_date TYPE date USING signup_date::date;`

Only include steps that match the actual columns. If a column you'd clean does
not exist in the schema, omit that step — never invent columns.

## Runtime Assumptions
- PostgreSQL reachable via `psql` (local) or inside the sandbox Postgres container.

## Constraints
- Never mutates the raw source table — all output goes to `clean_<table>`.
- Statements run sequentially; the first failure stops the run and is returned.

## Common Patterns
### Ingestion Cleanup
Kafka Consumer → Raw Database Table → **Data Cleaning (`run_cleaning`)** → `clean_` Table → Spark Aggregator → Report

## Roadmap (not yet available)
CSV / non-SQL sources: a future `run_cleaning` extension will accept a `source`
descriptor (e.g. a CSV path) and stage it into a table first. For now, source
data must already be a Postgres table.
