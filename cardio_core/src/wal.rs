//! Write-Ahead Log (WAL) for session persistence.
//!
//! Sessions are append to a JSONL (JSON Lines) file with file locking
//! to ensure safe concurrent access.

use crate::{MicrodoseSession, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Session sink trait for persisting sessions
pub trait SessionSink {
    fn append(&mut self, session: &MicrodoseSession) -> Result<()>;
}

/// JSONL-based session sink with file locking
pub struct JsonlSink {
    path: PathBuf,
}

impl JsonlSink {
    /// Create a new JSONL sink for the given path
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Ensure the parent directory exists
    fn ensure_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

impl SessionSink for JsonlSink {
    fn append(&mut self, session: &MicrodoseSession) -> Result<()> {
        self.ensure_parent_dir()?;

        // Open file for appending
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        // Acquire exclusive lock
        file.lock_exclusive()?;

        // Write session as JSON line
        let mut writer = std::io::BufWriter::new(&file);
        let line = serde_json::to_string(session)?;
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;

        // Lock is automatically released when file is dropped
        file.unlock()?;

        tracing::debug!("Appended session {} to WAL", session.id);
        Ok(())
    }
}

/// Read all sessions from a WAL file
pub fn read_sessions(path: &Path) -> Result<Vec<MicrodoseSession>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(path)?;
    // Acquire shared lock for reading
    file.lock_shared()?;

    let reader = BufReader::new(&file);
    let mut sessions = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<MicrodoseSession>(&line) {
            Ok(session) => sessions.push(session),
            Err(e) => {
                tracing::warn!("Failed to parse session at line {}: {}", line_num + 1, e);
                // Continue reading, don't fail completely
            }
        }
    }

    file.unlock()?;
    tracing::debug!("Read {} sessions from WAL", sessions.len());
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_session() -> MicrodoseSession {
        MicrodoseSession {
            id: Uuid::new_v4(),
            definition_id: "test_def".into(),
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
    fn test_append_and_read_single_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let session = create_test_session();
        let session_id = session.id;

        // Append session
        let mut sink = JsonlSink::new(&wal_path);
        sink.append(&session).unwrap();

        // Read back
        let sessions = read_sessions(&wal_path).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session_id);
    }

    #[test]
    fn test_append_multiple_sessions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let mut sink = JsonlSink::new(&wal_path);

        // Append multiple sessions
        for _ in 0..5 {
            let session = create_test_session();
            sink.append(&session).unwrap();
        }

        // Read back
        let sessions = read_sessions(&wal_path).unwrap();
        assert_eq!(sessions.len(), 5);
    }

    #[test]
    fn test_read_empty_wal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("nonexistent.wal");

        let sessions = read_sessions(&wal_path).unwrap();
        assert!(sessions.is_empty());
    }
}
