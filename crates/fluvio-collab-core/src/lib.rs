//! # fluvio-collab-core
//!
//! Collaboration logic: clients for the other Fluvio services, policy, and
//! workflow orchestration. Pure library — no transport, no env reads. The
//! GraphQL subgraph that exposes it lives in `servers/collab-server`.

pub mod clients;
pub mod policy;
pub mod workflows;
