//! # database-server
//!
//! Transport shell for the [`fluvio_database`] crate: an axum + async-graphql
//! subgraph. Config is read from the environment here, never in the library.

pub mod graphql;
pub mod internal;
pub mod server;
