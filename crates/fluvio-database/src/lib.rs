//! # fluvio-database
//!
//! Postgres persistence for Fluvio: connection pools, migrations, and typed
//! queries. Pure library — no transport, no env reads. The GraphQL subgraph
//! that exposes it lives in `servers/database-server`.

pub mod db;
