# Database SQL Tool

## Purpose
Provides read-only database query execution and metadata/schema discovery to inspect tables, structure, and sample data prior to data pipeline creation.

## Supported Capabilities
- List available tables
- Get column schema and type definitions
- Run read-only SELECT queries with row limits

## Runtime Assumptions
- PostgreSQL running locally
- Native `psql` binary available on PATH

## Safety Constraints
- **Read-Only**: Only SELECT queries are permitted.
- Destructive SQL statements (e.g. `DROP`, `DELETE`, `UPDATE`, `INSERT`, `ALTER`, `TRUNCATE`, `CREATE`, `GRANT`) are blocked.

## Recommended Usage
Use this tool to:
- Verify column names and data types of source tables
- Run test query joins to confirm data relationships
- Read sample rows to understand formatting (e.g., date formats, currency types)
