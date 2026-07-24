//! # fluvio-graph-core
//!
//! The knowledge graph engine: SurrealDB-backed storage, embeddings, retrieval,
//! and traversal. Pure library — no transport, no env reads. The GraphQL
//! subgraph that exposes it lives in `servers/graph-server`.

pub mod embeddings;
pub mod graph;
pub mod query_context;
pub mod registry;
pub mod storage;
