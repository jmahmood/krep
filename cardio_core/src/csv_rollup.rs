//! CSV rollup functionality for archiving WAL sessions.
//!
//! This module implements atomic WAL-to-CSV conversion with proper error handling
//! to prevent data loss.

use crate::{MicrodoseSession, Result};
use std::fs::OpenOptions;
use std::path::Path;

/// A row in the CSV output
#[derive(Debug, serde::Serialize)]
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

impl From<&MicrodoseSession> for CsvRow {
    fn from(session: &MicrodoseSession) -> Self {
        CsvRow {
            id: session.id.to_string(),
            definition_id: session.definition_id.clone(),
            performed_at: session.performed_at.to_rfc3339(),
            started_at: session.started_at.map(|t| t.to_rfc3339()),
            completed_at: session.completed_at.map(|t| t.to_rfc3339()),
            duration: session.actual_duration_seconds,
            perceived_rpe: session.perceived_rpe,
            avg_hr: session.avg_hr,
            max_hr: session.max_hr,
        }
    }
}

/// Roll up WAL sessions into CSV and archive the WAL atomically
///
/// This function:
/// 1. Reads all sessions from the WAL
/// 2. Appends them to the CSV file (creates with headers if needed)
/// 3. Syncs the CSV to disk
/// 4. Renames the WAL to .processed
/// 5. Returns the number of sessions processed
///
/// # Safety
/// - CSV is fsynced before WAL is renamed
/// - WAL is renamed (not deleted) to allow manual recovery if needed
/// - Processed WAL files can be cleaned up manually
pub fn wal_to_csv_and_archive(wal_path: &Path, csv_path: &Path) -> Result<usize> {
    // Read all sessions from WAL
    let sessions = crate::wal::read_sessions(wal_path)?;

    if sessions.is_empty() {
        tracing::info!("No sessions in WAL to roll up");
        return Ok(0);
    }

    // Ensure parent directory exists
    if let Some(parent) = csv_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open CSV file for appending
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(csv_path)?;

    // Determine if we need to write headers by checking file size after opening
    // This avoids an extra stat() syscall
    let needs_headers = file.metadata()?.len() == 0;

    // CSV writer automatically writes headers if the serialized type has them
    // For appending, we need to skip headers manually if file already has content
    let mut writer = csv::WriterBuilder::new()
        .has_headers(needs_headers)
        .from_writer(file);

    // Write all sessions to CSV
    for session in &sessions {
        let row = CsvRow::from(session);
        writer.serialize(row)?;
    }

    // Flush and sync to disk
    writer.flush()?;
    let file = writer
        .into_inner()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    file.sync_all()?;

    tracing::info!("Wrote {} sessions to CSV", sessions.len());

    // Atomically archive the WAL by renaming it
    let processed_path = wal_path.with_extension("wal.processed");
    std::fs::rename(wal_path, &processed_path)?;

    tracing::info!("Archived WAL to {:?}", processed_path);

    Ok(sessions.len())
}

/// Clean up old processed WAL files
///
/// This removes all .wal.processed files in the given directory.
pub fn cleanup_processed_wals(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(extension) = path.extension() {
            if extension == "processed" {
                std::fs::remove_file(&path)?;
                tracing::debug!("Removed processed WAL: {:?}", path);
                count += 1;
            }
        }
    }

    if count > 0 {
        tracing::info!("Cleaned up {} processed WAL files", count);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::SessionSink;
    use chrono::Utc;
    use std::fs::File;
    use uuid::Uuid;

    fn create_test_session(def_id: &str) -> MicrodoseSession {
        MicrodoseSession {
            id: Uuid::new_v4(),
            definition_id: def_id.into(),
            performed_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            actual_duration_seconds: Some(300),
            metrics_realized: vec![],
            perceived_rpe: Some(7),
            avg_hr: Some(145),
            max_hr: Some(165),
        }
    }

    #[test]
    fn test_wal_to_csv_creates_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("sessions.wal");
        let csv_path = temp_dir.path().join("sessions.csv");

        // Write sessions to WAL
        let mut sink = crate::wal::JsonlSink::new(&wal_path);
        for i in 0..3 {
            let session = create_test_session(&format!("def_{}", i));
            sink.append(&session).unwrap();
        }

        // Roll up to CSV
        let count = wal_to_csv_and_archive(&wal_path, &csv_path).unwrap();
        assert_eq!(count, 3);

        // Verify CSV exists
        assert!(csv_path.exists());

        // Verify WAL was archived
        assert!(!wal_path.exists());
        assert!(wal_path.with_extension("wal.processed").exists());
    }

    #[test]
    fn test_wal_to_csv_appends() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("sessions.wal");
        let csv_path = temp_dir.path().join("sessions.csv");

        // First rollup
        let mut sink = crate::wal::JsonlSink::new(&wal_path);
        sink.append(&create_test_session("def_1")).unwrap();
        let count1 = wal_to_csv_and_archive(&wal_path, &csv_path).unwrap();
        assert_eq!(count1, 1);

        // Second rollup (appends)
        let mut sink = crate::wal::JsonlSink::new(&wal_path);
        sink.append(&create_test_session("def_2")).unwrap();
        let count2 = wal_to_csv_and_archive(&wal_path, &csv_path).unwrap();
        assert_eq!(count2, 1);

        // Verify CSV has both entries
        let reader = csv::Reader::from_path(&csv_path).unwrap();
        let record_count = reader.into_records().count();
        assert_eq!(record_count, 2);
    }

    #[test]
    fn test_empty_wal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("empty.wal");
        let csv_path = temp_dir.path().join("sessions.csv");

        // Create empty WAL
        File::create(&wal_path).unwrap();

        let count = wal_to_csv_and_archive(&wal_path, &csv_path).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_cleanup_processed_wals() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create some processed WAL files
        File::create(temp_dir.path().join("s1.wal.processed")).unwrap();
        File::create(temp_dir.path().join("s2.wal.processed")).unwrap();
        File::create(temp_dir.path().join("keep.wal")).unwrap();

        let count = cleanup_processed_wals(temp_dir.path()).unwrap();
        assert_eq!(count, 2);

        // Verify only .processed files were removed
        assert!(!temp_dir.path().join("s1.wal.processed").exists());
        assert!(!temp_dir.path().join("s2.wal.processed").exists());
        assert!(temp_dir.path().join("keep.wal").exists());
    }
}
