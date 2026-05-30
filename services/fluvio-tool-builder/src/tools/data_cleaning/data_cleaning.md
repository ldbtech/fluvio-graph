# Data Cleaning Processor

## Purpose
Cleans, normalizes, and standardizes database tables and columns to ensure data quality before downstream analytical queries, modeling, or reporting are executed.

## Supported Capabilities
- **Header Normalization**: Replaces mixed casing and spaces in column names with lowercase snake_case.
- **Null Purging**: Removes rows containing null values in mandatory fields (e.g., email, id, user_id).
- **Deduplication**: Removes duplicate records based on primary key constraints or row uniqueness.
- **Currency Standardization**: Normalizes currency formats into numeric base USD.

## Runtime Assumptions
- PostgreSQL running locally
- Native `psql` client available on PATH

## Constraints
- **Safety First**: Never modifies raw source tables. Always outputs cleaned data into a new table prefixed with `clean_` (e.g., `clean_users`).

## Common Patterns
### Ingestion Cleanup
Kafka Consumer → Raw Database Table → Data Cleaning Processor → Cleaned Table → Spark Aggregator
