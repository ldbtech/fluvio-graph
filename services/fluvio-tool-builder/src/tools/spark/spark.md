# Spark Tool

## Purpose
Provides distributed analytical processing, high-volume SQL query execution, and PySpark/Scala batch/streaming job orchestration.

## Supported Capabilities
- Execute Spark SQL queries
- Submit PySpark / Scala / Java JAR jobs
- Monitor Spark job status and executor lag

## Runtime Assumptions
- Docker runtime available
- Local cluster initially (single-node master/worker)
- Spark UI available on port 8080

## Dependencies
- Docker
- Spark base image (e.g. `bitnami/spark:3.5.1`)
- Data volume paths for files / logs

## Actions

### `execute_sql`
Runs a Spark SQL aggregation and materialises the result as a table.
Arguments:
- `context`: execution context (`app_name`, `master_url`, `sandbox_id`, `database_url`)
- `query`: the SELECT you author. **Write it against the ACTUAL schema** in the
  workspace context — reference only real tables and columns, and read from the
  `clean_*` tables produced by the data-cleaning step (never raw source tables).
- `output_table`: the destination table name. It **MUST end with `_analytics`**
  (e.g. `revenue_by_country_analytics`) so the reporting engine can discover it.

### `submit_job` / `get_job_status`
Submit a PySpark/JAR job and poll its status.

## Authoring SQL
You decide the analysis. Given the user's question (e.g. "user growth vs revenue
and profit"), author one `execute_sql` step per metric:
- derive monthly growth, revenue, profit, segmentation, etc. from the real columns
- always read from `clean_<table>` inputs
- always write to a `<metric>_analytics` output table

## Common Patterns
### BI Rollup Job
`clean_orders → spark.execute_sql (aggregation) → orders_revenue_analytics → dashboard-syncer`

### Real-Time Aggregator
`Kafka topic → Spark Structured Streaming → warehouse`

## Constraints
- Read from `clean_*` tables; write to `*_analytics` tables.
- Memory allocations limited in local mode.
