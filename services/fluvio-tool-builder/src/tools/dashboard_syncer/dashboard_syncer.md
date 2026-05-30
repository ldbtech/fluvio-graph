# Executive Dashboard Publisher

## Purpose
Deploys reports, registers datasets, and triggers refreshes on BI platforms (PowerBI and Tableau) to make analytical insights shareable and viewable online.

## Supported Capabilities
- **Publish Report**: Deploys a report template (.pbix or .twbx) to a target BI workspace.
- **Trigger Refresh**: Triggers a dataset/model refresh to sync with PostgreSQL or SurrealDB updates.
- **Generate Share Link**: Fetches secure embed and public sharing URLs for team dashboards.

## Runtime Assumptions
- BI cloud gateways reachable (PowerBI REST API or Tableau Cloud API).
- Valid Workspace/Site identifier provided.

## Dependencies
- HTTPx client.

## Common Patterns
### Metric Reporting Loop
Database SQL / Spark Aggregator → Database Metrics → Dashboard Syncer (Trigger Refresh) → Live Visuals Updated

## Constraints
- Requires configuration of API client secrets / tokens for actual cloud operations. Falls back to realistic sandbox simulation locally.
