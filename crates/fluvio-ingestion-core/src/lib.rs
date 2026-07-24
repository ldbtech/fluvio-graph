//! # fluvio-ingestion-core
//!
//! The ingestion pipeline: extract content from files and text, chunk it,
//! embed it, and write the resulting nodes to the graph. Pure library — no
//! transport, no env reads. The GraphQL subgraph that drives it lives in
//! `servers/ingestion-server`.

pub mod client;
pub mod extractor;
pub mod pipeline;
