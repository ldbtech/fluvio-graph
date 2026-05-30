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

## Common Patterns
### Real-Time Aggregator
Kafka Ingest Topic → Spark Structured Streaming → SurrealDB / Warehouse

### BI Rollup Job
Raw Transactions Table → Spark SQL (Aggregation) → Campaign Metrics Table

## Constraints
- Memory allocations limited in local mode
- Spark catalog tables require metadata storage config
