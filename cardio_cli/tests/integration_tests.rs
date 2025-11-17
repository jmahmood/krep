//! Integration tests for the cardio_cli binary.
//!
//! These tests verify end-to-end behavior including:
//! - Session logging workflow
//! - CSV rollup operations
//! - Data persistence and recovery
//! - Concurrency safety

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a test data directory
fn setup_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Helper to get the path to the CLI binary
fn cli() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("krep"))
}

#[test]
fn test_cli_help() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cardio microdose prescription system",
        ));
}

#[test]
fn test_default_command_creates_directories() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();

    // Verify directories were created
    assert!(data_dir.join("wal").exists());
    assert!(data_dir.join("wal/microdose_sessions.wal").exists());
    // Note: state.json is only created when needed (mobility or progression updates)
}

#[test]
fn test_session_logged_to_wal() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Run once to create a session
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success()
        .stdout(predicate::str::contains("Session logged"));

    // Verify WAL file has content
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_content = fs::read_to_string(&wal_path).expect("Failed to read WAL");
    assert!(!wal_content.is_empty());
    assert!(wal_content.contains("definition_id"));
}

#[test]
fn test_dry_run_does_not_log() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Run in dry-run mode
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dry run"));

    // Verify no WAL file was created
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    assert!(!wal_path.exists());
}

#[test]
fn test_category_override() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Force mobility category
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--category")
        .arg("mobility")
        .arg("--auto-complete")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mobility"));
}

#[test]
fn test_rollup_creates_csv() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create some sessions
    for _ in 0..3 {
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--auto-complete")
            .assert()
            .success();
    }

    // Run rollup
    cli()
        .arg("rollup")
        .arg("--data-dir")
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("Rolled up 3 sessions"));

    // Verify CSV was created
    let csv_path = data_dir.join("sessions.csv");
    assert!(csv_path.exists());

    let csv_content = fs::read_to_string(&csv_path).expect("Failed to read CSV");
    assert!(csv_content.contains("id,definition_id"));
}

#[test]
fn test_rollup_with_cleanup() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create session
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();

    // Run rollup with cleanup
    cli()
        .arg("rollup")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--cleanup")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleaned up 1 processed WAL"));

    // Verify processed WAL was removed
    let wal_dir = data_dir.join("wal");
    let entries: Vec<_> = fs::read_dir(&wal_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".wal.processed"))
        .collect();

    assert_eq!(entries.len(), 0);
}

#[test]
fn test_multiple_sessions_round_robin() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    let mut categories = Vec::new();

    // Run 6 sessions to see round-robin behavior
    for _ in 0..6 {
        let output = cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--auto-complete")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let stdout = String::from_utf8_lossy(&output);

        if stdout.contains("Vo2 MICRODOSE") {
            categories.push("Vo2");
        } else if stdout.contains("Gtg MICRODOSE") {
            categories.push("Gtg");
        } else if stdout.contains("Mobility MICRODOSE") {
            categories.push("Mobility");
        }
    }

    // Should see all three categories
    assert!(categories.contains(&"Vo2"));
    assert!(categories.contains(&"Gtg"));
    assert!(categories.contains(&"Mobility"));
}

#[test]
fn test_state_persistence_across_runs() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // First run - force mobility to create state
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--category")
        .arg("mobility")
        .arg("--auto-complete")
        .assert()
        .success();

    let state_path = data_dir.join("wal/state.json");
    assert!(state_path.exists());

    // Second run should not crash
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();
}

#[test]
fn test_empty_rollup() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create directories but no sessions
    fs::create_dir_all(data_dir.join("wal")).unwrap();

    // Rollup should not fail on empty WAL
    cli()
        .arg("rollup")
        .arg("--data-dir")
        .arg(&data_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("nothing to roll up"));
}

#[test]
fn test_invalid_category_falls_back() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Use invalid category
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--category")
        .arg("invalid_category")
        .arg("--auto-complete")
        .assert()
        .success()
        .stderr(predicate::str::contains("Unknown category"));
}
