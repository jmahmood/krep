//! User state persistence with file locking.
//!
//! This module handles saving and loading user progression state
//! with proper file locking to prevent concurrent access issues.

use crate::{Error, Result, UserMicrodoseState};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

impl UserMicrodoseState {
    /// Load user state from a file with shared locking
    ///
    /// Returns default state if file doesn't exist.
    /// If file is corrupted, returns an error.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::info!("No state file found, using default state");
            return Ok(Self::default());
        }

        let file = File::open(path)?;

        // Acquire shared lock for reading
        file.lock_shared()?;

        let mut contents = String::new();
        let mut reader = std::io::BufReader::new(&file);
        reader.read_to_string(&mut contents)?;

        file.unlock()?;

        match serde_json::from_str::<UserMicrodoseState>(&contents) {
            Ok(state) => {
                tracing::debug!("Loaded user state from {:?}", path);
                Ok(state)
            }
            Err(e) => {
                tracing::error!("Failed to parse state file: {}", e);
                Err(Error::State(format!(
                    "Corrupted state file: {}. Consider deleting it to reset.",
                    e
                )))
            }
        }
    }

    /// Save user state to a file with exclusive locking
    ///
    /// Atomically writes state by:
    /// 1. Writing to a temp file
    /// 2. Syncing to disk
    /// 3. Renaming over the original
    pub fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)?;

        // Acquire exclusive lock
        file.lock_exclusive()?;

        {
            let mut writer = std::io::BufWriter::new(&file);
            let contents = serde_json::to_string_pretty(self)?;
            writer.write_all(contents.as_bytes())?;
            writer.flush()?;
        } // writer dropped here, releasing borrow

        // Sync to disk before rename
        file.sync_all()?;
        file.unlock()?;

        // Atomically replace old state file
        std::fs::rename(&temp_path, path)?;

        tracing::debug!("Saved user state to {:?}", path);
        Ok(())
    }

    /// Load state, modify it, and save it back atomically
    ///
    /// This is a convenience method that handles the load-modify-save pattern
    /// with proper error handling.
    pub fn update<F>(path: &Path, f: F) -> Result<Self>
    where
        F: FnOnce(&mut UserMicrodoseState) -> Result<()>,
    {
        let mut state = Self::load(path)?;
        f(&mut state)?;
        state.save(path)?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MovementStyle, ProgressionState};
    use chrono::Utc;

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("state.json");

        let mut state = UserMicrodoseState::default();
        state.progressions.insert(
            "emom_burpee_5m".into(),
            ProgressionState {
                reps: 5,
                style: MovementStyle::None,
                level: 2,
                last_upgraded: Some(Utc::now()),
            },
        );
        state.last_mobility_def_id = Some("mobility_hip_cars".into());

        // Save
        state.save(&state_path).unwrap();

        // Load
        let loaded = UserMicrodoseState::load(&state_path).unwrap();

        assert_eq!(loaded.progressions.len(), 1);
        assert!(loaded.progressions.contains_key("emom_burpee_5m"));
        assert_eq!(loaded.last_mobility_def_id, Some("mobility_hip_cars".into()));
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("nonexistent.json");

        let state = UserMicrodoseState::load(&state_path).unwrap();
        assert!(state.progressions.is_empty());
        assert_eq!(state.last_mobility_def_id, None);
    }

    #[test]
    fn test_update_pattern() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("state.json");

        // Initialize empty state
        UserMicrodoseState::default().save(&state_path).unwrap();

        // Update using the update helper
        UserMicrodoseState::update(&state_path, |state| {
            state.last_mobility_def_id = Some("test_mobility".into());
            Ok(())
        })
        .unwrap();

        // Verify update persisted
        let loaded = UserMicrodoseState::load(&state_path).unwrap();
        assert_eq!(loaded.last_mobility_def_id, Some("test_mobility".into()));
    }

    #[test]
    fn test_corrupted_state_returns_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("corrupted.json");

        // Write invalid JSON
        std::fs::write(&state_path, "{ invalid json }").unwrap();

        let result = UserMicrodoseState::load(&state_path);
        assert!(result.is_err());

        match result {
            Err(Error::State(_)) => {} // Expected
            _ => panic!("Expected State error"),
        }
    }

    #[test]
    fn test_atomic_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let state_path = temp_dir.path().join("state.json");

        let state = UserMicrodoseState::default();
        state.save(&state_path).unwrap();

        // Verify no temp file left behind
        assert!(!temp_dir.path().join("state.tmp").exists());
        assert!(state_path.exists());
    }
}
