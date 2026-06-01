# dbt CLI Builder

## Purpose
Runs, tests, and compiles SQL transformations in the data warehouse using dbt.

## Supported Capabilities
- Run dbt models with optional select or exclude flags.
- Execute data quality tests defined in the dbt project.
- Compile SQL compilation templates to dry-run transformations.

## Runtime Assumptions
- dbt is installed in the python environment.
- Profiles and project configuration exist in the directory specified.

## Dependencies
- dbt-core
- dbt database adapter (e.g., dbt-postgres, dbt-redshift)

## Common Patterns
- **Transform Pipeline**: Ingest data → Clean data → Run dbt models to transform → Sync to dashboard.
- **Data Quality Gate**: Run dbt models → Run dbt tests → Fail pipeline if test errors are found.

## Recommended Usage
Use dbt for managing SQL-based relational transformations, data quality verification, and documentation inside database engines.
