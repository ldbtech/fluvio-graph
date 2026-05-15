//! Structured logging initialisation for Fluvio microservices.
//!
//! Call `init_tracing()` once at the top of `main()` after `load_env()`.
//!
//! ## Log levels (controlled by `RUST_LOG` env var)
//! ```
//! RUST_LOG=info          # recommended for development
//! RUST_LOG=debug         # verbose, shows query plans and DB statements
//! RUST_LOG=warn          # production default (less noise)
//! RUST_LOG=fluvio=debug  # debug only Fluvio crates, silence dependencies
//! ```

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialise the global tracing subscriber.
///
/// - Format: human-readable in dev, structured JSON in prod
///   (controlled by `LOG_FORMAT` env var: `"pretty"` | `"json"`)
/// - Level: from `RUST_LOG` env var, defaulting to `"info"`
///
/// # Panics
/// Panics if called more than once (tracing-subscriber limitation).
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let log_format = std::env::var("LOG_FORMAT")
        .unwrap_or_else(|_| "pretty".to_string());

    match log_format.as_str() {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().json())
                .init();
        }
        _ => {
            // "pretty" — default for local dev
            tracing_subscriber::registry()
                .with(env_filter)
                .with(fmt::layer().with_target(true).with_thread_ids(false))
                .init();
        }
    }

    tracing::info!("Tracing initialised (format={})", log_format);
}