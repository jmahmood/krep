//! External strength training signal loader.
//!
//! This module loads strength training information from an external file
//! to inform microdose prescription decisions.

use crate::{ExternalStrengthSignal, Result, StrengthSessionType};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::Path;

/// Strength signal file format (matches external system output)
#[derive(Debug, Deserialize)]
struct StrengthSignalFile {
    last_session_at: DateTime<Utc>,
    session_type: String,
}

/// Load external strength training signal from a JSON file
///
/// Returns None if file doesn't exist (user hasn't logged strength training).
/// Returns an error if file exists but is malformed.
pub fn load_external_strength(path: &Path) -> Result<Option<ExternalStrengthSignal>> {
    if !path.exists() {
        tracing::debug!("No strength signal file found at {:?}", path);
        return Ok(None);
    }

    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(e) => {
            tracing::warn!(
                "Failed to read strength signal at {:?}: {}. Ignoring signal.",
                path,
                e
            );
            return Ok(None);
        }
    };

    let file: StrengthSignalFile = match serde_json::from_str(&contents) {
        Ok(file) => file,
        Err(e) => {
            tracing::warn!(
                "Failed to parse strength signal at {:?}: {}. Ignoring signal.",
                path,
                e
            );
            return Ok(None);
        }
    };

    let session_type = parse_session_type(&file.session_type);

    tracing::info!(
        "Loaded strength signal: {:?} at {}",
        session_type,
        file.last_session_at
    );

    Ok(Some(ExternalStrengthSignal {
        last_session_at: file.last_session_at,
        session_type,
    }))
}

/// Parse session type string into enum
fn parse_session_type(s: &str) -> StrengthSessionType {
    match s.to_lowercase().as_str() {
        "lower" => StrengthSessionType::Lower,
        "upper" => StrengthSessionType::Upper,
        "full" | "full_body" | "fullbody" => StrengthSessionType::Full,
        other => StrengthSessionType::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_strength_signal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let signal_path = temp_dir.path().join("strength.json");

        let json = r#"{
            "last_session_at": "2024-01-15T10:30:00Z",
            "session_type": "lower"
        }"#;

        std::fs::write(&signal_path, json).unwrap();

        let signal = load_external_strength(&signal_path).unwrap();
        assert!(signal.is_some());

        let signal = signal.unwrap();
        assert_eq!(signal.session_type, StrengthSessionType::Lower);
    }

    #[test]
    fn test_load_nonexistent_returns_none() {
        let temp_dir = tempfile::tempdir().unwrap();
        let signal_path = temp_dir.path().join("nonexistent.json");

        let signal = load_external_strength(&signal_path).unwrap();
        assert!(signal.is_none());
    }

    #[test]
    fn test_parse_session_types() {
        assert_eq!(parse_session_type("lower"), StrengthSessionType::Lower);
        assert_eq!(parse_session_type("UPPER"), StrengthSessionType::Upper);
        assert_eq!(parse_session_type("full"), StrengthSessionType::Full);
        assert_eq!(parse_session_type("full_body"), StrengthSessionType::Full);

        match parse_session_type("custom_session") {
            StrengthSessionType::Other(s) => assert_eq!(s, "custom_session"),
            _ => panic!("Expected Other variant"),
        }
    }

    #[test]
    fn test_malformed_json_returns_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let signal_path = temp_dir.path().join("bad.json");

        std::fs::write(&signal_path, "{ invalid json }").unwrap();

        let result = load_external_strength(&signal_path);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_upper_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let signal_path = temp_dir.path().join("strength.json");

        let json = r#"{
            "last_session_at": "2024-01-15T14:00:00Z",
            "session_type": "upper"
        }"#;

        std::fs::write(&signal_path, json).unwrap();

        let signal = load_external_strength(&signal_path).unwrap().unwrap();
        assert_eq!(signal.session_type, StrengthSessionType::Upper);
    }
}
