//! Environment variable loading helpers.
//!
//! Every microservice calls `load_env()` at the top of `main()`,
//! then uses `require_var()` to get values with clear error messages.

/// Load `.env` file into the process environment.
///
/// - In development: reads `.env` from the current working directory.
/// - In production (Docker): env vars are injected by the container runtime;
///   `.env` won't exist and `dotenvy` silently skips it — this is correct.
///
/// Call this **once** at the top of `main()` before any other setup.
pub fn load_env() {
    match dotenvy::dotenv() {
        Ok(path) => tracing::debug!("Loaded env from {:?}", path),
        Err(dotenvy::Error::Io(_)) => {
            // No .env file — expected in production, fine to ignore.
            tracing::debug!("No .env file found — using process environment");
        }
        Err(e) => {
            // Malformed .env — surface this as a warning, don't panic.
            tracing::warn!("Failed to parse .env file: {e}");
        }
    }
}

/// Read a required environment variable.
///
/// Panics with a clear message if the variable is missing.
/// Use this for values that are genuinely required for the service to start
/// (DB URLs, JWT secrets, ports). For optional values use `std::env::var()` directly.
///
/// # Panics
/// Panics if the environment variable is not set.
pub fn require_var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        panic!(
            "Required environment variable '{key}' is not set.\n\
             Check your .env file or container environment.\n\
             See .env.example for the full list of required variables."
        )
    })
}

/// Read an optional environment variable with a default.
pub fn var_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Read a required `u16` environment variable (typically `PORT`).
///
/// # Panics
/// Panics if the variable is missing or not a valid u16.
pub fn require_port(key: &str) -> u16 {
    let raw = require_var(key);
    raw.parse::<u16>().unwrap_or_else(|_| {
        panic!("Environment variable '{key}' must be a valid port number (0–65535), got: '{raw}'")
    })
}
