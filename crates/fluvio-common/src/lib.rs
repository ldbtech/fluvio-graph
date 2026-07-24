//! # fluvio-common
//!
//! Shared infrastructure utilities used by every Fluvio microservice.
//!
//! ## What lives here
//! - [`error`] — `AppError`, the unified error type (implements
//!   `axum::response::IntoResponse` when the `server` feature is enabled)
//! - [`tracing`] — `init_tracing()` sets up structured JSON logging consistently
//! - [`config`] — `require_var()` and `load_env()` for env var loading with clear error messages

#![forbid(unsafe_code)]
#![warn(clippy::all)]

pub mod config;
pub mod error;
pub mod tracing;

pub use config::{load_env, require_var};
pub use error::AppError;
pub use tracing::init_tracing;