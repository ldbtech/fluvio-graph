//! fluvio-ingestion library root.
pub mod client;
pub mod extractor;
#[cfg(feature = "server")]
pub mod graphql;
pub mod pipeline;
#[cfg(feature = "server")]
pub mod server;