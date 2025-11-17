//! Error types for the cardio_core library.

use std::io;

/// Result type alias using our Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Core error type for cardio_core operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// IO error occurred
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// CSV error
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// TOML parsing error
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Configuration validation error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Catalog validation error
    #[error("Catalog validation error: {0}")]
    CatalogValidation(String),

    /// State management error
    #[error("State error: {0}")]
    State(String),

    /// Prescription engine error
    #[error("Prescription error: {0}")]
    Prescription(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}
