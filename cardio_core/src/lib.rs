#![forbid(unsafe_code)]

//! Core domain model and business logic for the Cardio Microdose system.
//!
//! This crate provides:
//! - Domain types (movements, microdoses, sessions, metrics)
//! - Catalog management
//! - Prescription engine
//! - Persistence (WAL, CSV, state)
//! - Progression logic

pub mod catalog;
pub mod config;
pub mod csv_rollup;
pub mod engine;
pub mod error;
pub mod history;
pub mod logging;
pub mod progression;
pub mod state;
pub mod strength;
pub mod types;
pub mod wal;

// Re-export commonly used types
pub use catalog::build_default_catalog;
pub use config::Config;
pub use engine::{prescribe_next, PrescribedMicrodose};
pub use error::{Error, Result};
pub use history::load_recent_sessions;
pub use progression::increase_intensity;
pub use strength::load_external_strength;
pub use types::*;
pub use wal::{JsonlSink, SessionSink};
