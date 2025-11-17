#![forbid(unsafe_code)]

//! Core domain model and business logic for the Cardio Microdose system.
//!
//! This crate provides:
//! - Domain types (movements, microdoses, sessions, metrics)
//! - Catalog management
//! - Prescription engine
//! - Persistence (WAL, CSV, state)
//! - Progression logic

pub mod types;
pub mod error;
pub mod catalog;
pub mod config;
pub mod logging;
pub mod wal;
pub mod csv_rollup;
pub mod state;
pub mod strength;
pub mod history;
pub mod progression;
pub mod engine;

// Re-export commonly used types
pub use error::{Error, Result};
pub use types::*;
pub use catalog::build_default_catalog;
pub use config::Config;
pub use wal::{JsonlSink, SessionSink};
pub use strength::load_external_strength;
pub use history::load_recent_sessions;
pub use progression::increase_intensity;
pub use engine::{prescribe_next, PrescribedMicrodose};
