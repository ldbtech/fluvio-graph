//! # fluvio-common
//!
//! Shared infrastructure utilities used by every Fluvio microservice.
//!
//! ## What lives here
//! - [`error`] — `AppError`, the unified error type (implements
//!   `axum::response::IntoResponse` when the `server` feature is enabled)
//! - [`tracing`] — `init_tracing()` sets up structured JSON logging consistently
//!   (binary-side; requires the `env` feature)
//! - [`config`] — `require_var()` and `load_env()` for env var loading with clear
//!   error messages (binary-side; requires the `env` feature)
//!
//! Library code must never read the environment or install a tracing
//! subscriber — the `env`-featured modules exist for **binaries** only.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

#[cfg(feature = "env")]
pub mod config;
pub mod error;
#[cfg(feature = "env")]
pub mod tracing;

#[cfg(feature = "env")]
pub use config::{load_env, require_var};
pub use error::AppError;
#[cfg(feature = "env")]
pub use tracing::init_tracing;