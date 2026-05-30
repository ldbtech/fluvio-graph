# Kafka Tool

## Purpose
Provides event streaming infrastructure and messaging capabilities.

## Supported Capabilities
- Create Kafka clusters
- Create topics
- Produce messages
- Consume messages
- Monitor lag
- Stream analytics integration

## Runtime Assumptions
- Docker runtime available
- Local mode initially
- Single-node Kafka supported

## Dependencies
- Docker
- Network bridge
- Persistent storage

## Common Patterns
### Event Streaming
Producer → Kafka Topic → Spark Consumer

### CDC Pipeline
Postgres → Kafka → Warehouse

## Constraints
- Topic names must be unique
- Replication factor limited in local mode

## Recommended Usage
Use Kafka for:
- streaming pipelines
- async processing
- event sourcing
- real-time analytics