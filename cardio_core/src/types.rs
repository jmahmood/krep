//! Core domain types for the Cardio Microdose system.
//!
//! This module defines the fundamental types used throughout the system:
//! - Movements and their properties
//! - Metrics (reps, bands, etc.)
//! - Microdose definitions and sessions
//! - User state and progression tracking
//! - Strength signal integration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Movement Types
// ============================================================================

/// Type of movement/exercise
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MovementKind {
    KettlebellSwing,
    Burpee,
    Pullup,
    MobilityDrill,
}

/// Burpee variation styles
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BurpeeStyle {
    FourCount,
    SixCount,
    SixCountTwoPump,
    Seal,
}

/// Specification for resistance bands
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BandSpec {
    None,
    NamedColour(String),
}

/// Style variations for movements
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MovementStyle {
    None,
    Burpee(BurpeeStyle),
    Band(BandSpec),
}

/// A movement definition (e.g., "Kettlebell Swing")
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Movement {
    pub id: String,
    pub name: String,
    pub kind: MovementKind,
    pub default_style: MovementStyle,
    pub tags: Vec<String>,
    pub reference_url: Option<String>,
}

// ============================================================================
// Metric Types (v1.1 enum-based design)
// ============================================================================

/// Metric specification with type-safe variants
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricSpec {
    /// Repetition-based metric (e.g., number of swings, burpees)
    Reps {
        key: String,
        default: i32,
        min: i32,
        max: i32,
        step: i32,
        progressable: bool,
    },
    /// Band specification metric (e.g., pullup assistance band)
    Band {
        key: String,
        default: String,
        progressable: bool,
    },
}

// ============================================================================
// Microdose Block and Definition Types
// ============================================================================

/// A single work block within a microdose (e.g., one EMOM interval)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicrodoseBlock {
    pub movement_id: String,
    pub movement_style: MovementStyle,
    pub duration_hint_seconds: u32,
    pub metrics: Vec<MetricSpec>,
}

/// Category of microdose workout
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MicrodoseCategory {
    Vo2,
    Gtg,
    Mobility,
}

/// A complete microdose workout definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicrodoseDefinition {
    pub id: String,
    pub name: String,
    pub category: MicrodoseCategory,
    pub suggested_duration_seconds: u32,
    pub gtg_friendly: bool,
    pub blocks: Vec<MicrodoseBlock>,
    pub reference_url: Option<String>,
}

// ============================================================================
// Session and State Types
// ============================================================================

/// A recorded microdose session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicrodoseSession {
    pub id: Uuid,
    pub definition_id: String,
    pub performed_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub actual_duration_seconds: Option<u32>,
    pub metrics_realized: Vec<MetricSpec>,
    pub perceived_rpe: Option<u8>,
    pub avg_hr: Option<u8>,
    pub max_hr: Option<u8>,
}

/// Type-level distinction between real sessions and skipped prescriptions
///
/// This ensures that skipped sessions (used only for influencing the prescription
/// engine) can never accidentally reach persistence layers (WAL, CSV, state).
#[derive(Clone, Debug)]
pub enum SessionKind {
    /// A real session that was performed and should be persisted
    Real(MicrodoseSession),
    /// A prescription that was shown but skipped (in-memory only)
    ShownButSkipped {
        definition_id: String,
        shown_at: DateTime<Utc>,
    },
}

impl SessionKind {
    /// Get the definition ID for this session (works for both Real and ShownButSkipped)
    pub fn definition_id(&self) -> &str {
        match self {
            SessionKind::Real(session) => &session.definition_id,
            SessionKind::ShownButSkipped { definition_id, .. } => definition_id,
        }
    }

    /// Get the timestamp when this session/prescription occurred
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            SessionKind::Real(session) => session.performed_at,
            SessionKind::ShownButSkipped { shown_at, .. } => *shown_at,
        }
    }

    /// Check if this is a Real session (returns None for ShownButSkipped)
    pub fn as_real(&self) -> Option<&MicrodoseSession> {
        match self {
            SessionKind::Real(session) => Some(session),
            SessionKind::ShownButSkipped { .. } => None,
        }
    }
}

/// Progression state for a specific microdose definition (v1.1 improved design)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProgressionState {
    pub reps: i32,
    pub style: MovementStyle,
    pub level: u32,
    pub last_upgraded: Option<DateTime<Utc>>,
}

/// User's persistent state across sessions
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct UserMicrodoseState {
    pub progressions: HashMap<String, ProgressionState>,
    pub last_mobility_def_id: Option<String>,
}

/// Type of strength training session
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrengthSessionType {
    Lower,
    Upper,
    Full,
    Other(String),
}

/// External strength training signal (from another system)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalStrengthSignal {
    pub last_session_at: DateTime<Utc>,
    pub session_type: StrengthSessionType,
}

/// Runtime context for prescription engine
#[derive(Clone, Debug)]
pub struct UserContext {
    pub now: DateTime<Utc>,
    pub user_state: UserMicrodoseState,
    pub recent_sessions: Vec<SessionKind>,
    pub external_strength: Option<ExternalStrengthSignal>,
    pub equipment_available: Vec<String>,
}

// ============================================================================
// Catalog Type
// ============================================================================

/// The complete catalog of movements and microdose definitions
#[derive(Clone, Debug)]
pub struct Catalog {
    pub movements: HashMap<String, Movement>,
    pub microdoses: HashMap<String, MicrodoseDefinition>,
}
