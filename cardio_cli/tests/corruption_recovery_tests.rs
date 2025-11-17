//! Corruption recovery tests for cardio_cli.
//!
//! These tests verify the system can handle:
//! - Corrupted state files
//! - Corrupted WAL files
//! - Missing files
//! - Partial writes

use assert_cmd::Command;
use std::fs;
use std::io::Write as IoWrite;
use tempfile::TempDir;

fn cli() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("krep"))
}

fn setup_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

#[test]
fn test_corrupted_state_file() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create wal directory
    fs::create_dir_all(data_dir.join("wal")).unwrap();

    // Write corrupted state file
    let state_path = data_dir.join("wal/state.json");
    fs::write(&state_path, "{ invalid json }}}}").expect("Failed to write corrupted state");

    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();
}

#[test]
fn test_corrupted_wal_file_ignored_during_read() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create wal directory
    fs::create_dir_all(data_dir.join("wal")).unwrap();

    // Write corrupted WAL file (invalid JSON lines)
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    fs::write(&wal_path, "{ invalid json }\n{ more invalid }")
        .expect("Failed to write corrupted WAL");

    // CLI can still prescribe (corrupted lines are logged as warnings)
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--dry-run") // Don't write, just test reading
        .assert()
        .success();
}

#[test]
fn test_partial_wal_line() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create a WAL file with a partial last line (simulating crash during write)
    fs::create_dir_all(data_dir.join("wal")).unwrap();
    let wal_path = data_dir.join("wal/microdose_sessions.wal");

    let mut file = fs::File::create(&wal_path).unwrap();
    // Write valid line
    writeln!(
        file,
        r#"{{"id":"00000000-0000-0000-0000-000000000000","definition_id":"test"}}"#
    )
    .unwrap();
    // Write partial line (no newline)
    write!(file, r#"{{"id":"partial"#).unwrap();
    drop(file);

    // CLI should handle this gracefully
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();
}

#[test]
fn test_missing_strength_signal() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Don't create strength signal file - CLI should work fine
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();
}

#[test]
fn test_corrupted_strength_signal() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create strength directory with corrupted signal
    let strength_dir = data_dir.join("strength");
    fs::create_dir_all(&strength_dir).unwrap();

    let signal_path = strength_dir.join("signal.json");
    fs::write(&signal_path, "{ not valid json at all }").expect("Failed to write corrupted signal");

    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();
}

#[test]
fn test_empty_files() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create empty WAL (OK) but state file parsing requires valid JSON
    fs::create_dir_all(data_dir.join("wal")).unwrap();
    fs::write(data_dir.join("wal/microdose_sessions.wal"), "").unwrap();

    // CLI works with empty WAL (no state file created yet)
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();
}

#[test]
fn test_rollup_with_valid_wal() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create some valid sessions first
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();

    // Rollup should work
    cli()
        .arg("rollup")
        .arg("--data-dir")
        .arg(&data_dir)
        .assert()
        .success();

    // CSV should be created
    assert!(data_dir.join("sessions.csv").exists());
}

#[test]
fn test_state_manual_recovery() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create corrupted state
    fs::create_dir_all(data_dir.join("wal")).unwrap();
    let state_path = data_dir.join("wal/state.json");
    fs::write(&state_path, "corrupted").unwrap();

    // Runs should recover and proceed with defaults even when state is invalid
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--category")
        .arg("mobility")
        .arg("--auto-complete")
        .assert()
        .success();

    // Second run should still succeed (no manual recovery necessary)
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--category")
        .arg("mobility")
        .arg("--auto-complete")
        .assert()
        .success();

    // State file should now be valid
    let state_content = fs::read_to_string(&state_path).expect("State should exist");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&state_content);
    assert!(parsed.is_ok(), "State should be valid JSON");
}

#[test]
fn test_permission_denied_state() {
    // Skip on Windows (permission model is different)
    if cfg!(windows) {
        return;
    }

    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create state with invalid permissions
    fs::create_dir_all(data_dir.join("wal")).unwrap();
    let state_path = data_dir.join("wal/state.json");
    fs::write(&state_path, "{}").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&state_path).unwrap().permissions();
        perms.set_mode(0o000); // No permissions
        fs::set_permissions(&state_path, perms).unwrap();

        // CLI should handle permission error gracefully
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--dry-run")
            .assert()
            .success();

        // Should fail or warn about permissions
        // (exact behavior depends on error handling strategy)

        // Clean up permissions for temp dir cleanup
        let mut perms = fs::metadata(&state_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&state_path, perms).unwrap();
    }
}
