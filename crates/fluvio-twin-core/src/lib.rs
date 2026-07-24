//! # fluvio-twin-core
//!
//! Graph-grounded retrieval and LLM orchestration for the twin. Pure library —
//! no transport, no env reads. The GraphQL subgraph that exposes it lives in
//! `servers/twin-server`.

pub mod graph;
pub mod llm;
