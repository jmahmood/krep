//! Session history loading with 7-day window.
//!
//! This module loads recent session history from both WAL and CSV files
//! to provide context for the prescription engine.

use crate::{MicrodoseSession, Result};
use chrono::{DateTime, Duration, Utc};
use csv::ReaderBuilder;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;
use uuid::Uuid;

/// CSV row format for reading archived sessions
#[derive(Debug, Deserialize)]
struct CsvRow {
    id: String,
    definition_id: String,
    performed_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    duration: Option<u32>,
    perceived_rpe: Option<u8>,
    avg_hr: Option<u8>,
    max_hr: Option<u8>,
}

impl TryFrom<CsvRow> for MicrodoseSession {
    type Error = crate::Error;

    fn try_from(row: CsvRow) -> Result<Self> {
        let id = Uuid::parse_str(&row.id)
            .map_err(|e| crate::Error::Other(format!("Invalid UUID: {}", e)))?;

        let performed_at = DateTime::parse_from_rfc3339(&row.performed_at)
            .map_err(|e| crate::Error::Other(format!("Invalid date: {}", e)))?
            .with_timezone(&Utc);

        let started_at = row
            .started_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let completed_at = row
            .completed_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(MicrodoseSession {
            id,
            definition_id: row.definition_id,
            performed_at,
            started_at,
            completed_at,
            actual_duration_seconds: row.duration,
            metrics_realized: vec![], // Not stored in CSV
            perceived_rpe: row.perceived_rpe,
            avg_hr: row.avg_hr,
            max_hr: row.max_hr,
        })
    }
}

/// Load sessions from the last N days from both WAL and CSV
///
/// Returns sessions sorted by performed_at (newest first).
/// Automatically deduplicates sessions that appear in both WAL and CSV.
pub fn load_recent_sessions(
    wal_path: &Path,
    csv_path: &Path,
    days: i64,
) -> Result<Vec<MicrodoseSession>> {
    let cutoff = Utc::now() - Duration::days(days);
    let mut sessions = Vec::new();
    let mut seen_ids = HashSet::new();

    // Load from WAL first (most recent)
    if wal_path.exists() {
        let wal_sessions = crate::wal::read_sessions(wal_path)?;
        for session in wal_sessions {
            if session.performed_at >= cutoff {
                seen_ids.insert(session.id);
                sessions.push(session);
            }
        }
        tracing::debug!("Loaded {} sessions from WAL", sessions.len());
    }

    // Load from CSV (archived)
    if csv_path.exists() {
        let csv_sessions = load_sessions_from_csv(csv_path)?;
        let mut csv_count = 0;
        for session in csv_sessions {
            if session.performed_at >= cutoff && !seen_ids.contains(&session.id) {
                seen_ids.insert(session.id);
                sessions.push(session);
                csv_count += 1;
            }
        }
        tracing::debug!("Loaded {} sessions from CSV", csv_count);
    }

    // Sort by performed_at, newest first
    sessions.sort_by(|a, b| b.performed_at.cmp(&a.performed_at));

    tracing::info!(
        "Loaded {} total sessions from last {} days",
        sessions.len(),
        days
    );

    Ok(sessions)
}

/// Load all sessions from a CSV file
fn load_sessions_from_csv(path: &Path) -> Result<Vec<MicrodoseSession>> {
    let mut reader = ReaderBuilder::new().has_headers(true).from_path(path)?;

    let mut sessions = Vec::new();
    for result in reader.deserialize::<CsvRow>() {
        match result {
            Ok(row) => match MicrodoseSession::try_from(row) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    tracing::warn!("Failed to parse CSV row: {}", e);
                    // Continue processing other rows
                }
            },
            Err(e) => {
                tracing::warn!("Failed to deserialize CSV row: {}", e);
            }
        }
    }

    Ok(sessions)
}

/// Find the most recent session for a given category
pub fn find_last_session_by_category<'a>(
    sessions: &'a [MicrodoseSession],
    category: &str,
) -> Option<&'a MicrodoseSession> {
    // Sessions should already be sorted newest first
    sessions
        .iter()
        .find(|s| s.definition_id.contains(category))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::SessionSink;

    fn create_test_session(def_id: &str, days_ago: i64) -> MicrodoseSession {
        MicrodoseSession {
            id: Uuid::new_v4(),
            definition_id: def_id.into(),
            performed_at: Utc::now() - Duration::days(days_ago),
            started_at: Some(Utc::now() - Duration::days(days_ago)),
            completed_at: Some(Utc::now() - Duration::days(days_ago)),
            actual_duration_seconds: Some(300),
            metrics_realized: vec![],
            perceived_rpe: Some(7),
            avg_hr: Some(145),
            max_hr: Some(165),
        }
    }

    #[test]
    fn test_load_recent_sessions_from_wal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("sessions.wal");
        let csv_path = temp_dir.path().join("sessions.csv");

        // Create sessions at different days
        let mut sink = crate::wal::JsonlSink::new(&wal_path);
        sink.append(&create_test_session("vo2_1", 1)).unwrap();
        sink.append(&create_test_session("vo2_2", 3)).unwrap();
        sink.append(&create_test_session("vo2_3", 10)).unwrap(); // Too old

        let sessions = load_recent_sessions(&wal_path, &csv_path, 7).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_deduplication_across_wal_and_csv() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("sessions.wal");
        let csv_path = temp_dir.path().join("sessions.csv");

        // Add session to WAL
        let session = create_test_session("vo2_1", 1);
        let session_id = session.id;
        let mut sink = crate::wal::JsonlSink::new(&wal_path);
        sink.append(&session).unwrap();

        // Roll up to CSV (which includes the same session)
        crate::csv_rollup::wal_to_csv_and_archive(&wal_path, &csv_path).unwrap();

        // Load - should get only 1 session despite it being in CSV
        let sessions = load_recent_sessions(
            &temp_dir.path().join("nonexistent.wal"),
            &csv_path,
            7,
        )
        .unwrap();

        // Find the session
        let found = sessions.iter().find(|s| s.id == session_id);
        assert!(found.is_some());

        // Count how many times it appears (should be 1)
        let count = sessions.iter().filter(|s| s.id == session_id).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_sessions_sorted_newest_first() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("sessions.wal");
        let csv_path = temp_dir.path().join("sessions.csv");

        let mut sink = crate::wal::JsonlSink::new(&wal_path);
        let old = create_test_session("old", 5);
        let new = create_test_session("new", 1);

        // Add in reverse chronological order
        sink.append(&old).unwrap();
        sink.append(&new).unwrap();

        let sessions = load_recent_sessions(&wal_path, &csv_path, 7).unwrap();

        // Should be sorted newest first
        assert_eq!(sessions[0].definition_id, "new");
        assert_eq!(sessions[1].definition_id, "old");
    }

    #[test]
    fn test_find_last_session_by_category() {
        let s1 = create_test_session("emom_vo2", 3);
        let s2 = create_test_session("gtg_pullup", 2);
        let s3 = create_test_session("emom_vo2", 1);

        let sessions = vec![s3.clone(), s2, s1]; // Already sorted newest first

        let last_vo2 = find_last_session_by_category(&sessions, "vo2");
        assert!(last_vo2.is_some());
        assert_eq!(last_vo2.unwrap().id, s3.id);
    }
}
