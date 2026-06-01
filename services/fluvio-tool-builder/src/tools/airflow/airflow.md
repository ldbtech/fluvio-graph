# Apache Airflow Orchestrator

## Purpose
Orchestrates complex data workflows, triggers DAG runs, and monitors task execution.

## Supported Capabilities
- Trigger DAGs with custom configuration parameters.
- Retrieve the status of DAG runs.
- Monitor execution state (queued, running, success, failed).

## Runtime Assumptions
- Airflow webserver and scheduler are running in the target environment.
- REST API is accessible on port 8080 (default).

## Dependencies
- Apache Airflow 2.x
- REST API enabled on the Airflow instance.

## Common Patterns
- **Trigger Pipeline**: Run an Airflow DAG that orchestrates data cleaning, Spark analytics, and reporting.
- **Workflow Monitoring**: Poll the status of a long-running ingestion job.

## Recommended Usage
Use Airflow for scheduling, monitoring, and triggering multi-stage data orchestration pipelines.
