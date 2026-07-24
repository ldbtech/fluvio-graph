//! # twin-server
//!
//! Transport shell for [`fluvio_twin_core`]: an axum + async-graphql subgraph.
//! Config is read from the environment here, never in the library.

pub mod graphql;
pub mod server;
