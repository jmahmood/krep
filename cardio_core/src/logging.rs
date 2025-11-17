//! Logging infrastructure for Krep.
//!
//! Provides centralized tracing setup for all binaries.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging with sensible defaults
///
/// This sets up tracing with:
/// - Environment-based filtering (RUST_LOG)
/// - Colored output (if terminal supports it)
/// - Compact format
///
/// Default level is INFO, but can be overridden with RUST_LOG env var.
pub fn init() {
    init_with_level("info")
}

/// Initialize logging with a specific default level
///
/// # Arguments
/// * `default_level` - Default log level (debug, info, warn, error)
///
/// This can still be overridden by RUST_LOG environment variable.
pub fn init_with_level(default_level: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact())
        .init();
}

/// Initialize logging for testing (captures logs for test output)
#[cfg(test)]
pub fn init_test() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(EnvFilter::new("debug"))
        .try_init();
}
