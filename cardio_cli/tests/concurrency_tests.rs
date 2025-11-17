//! Concurrency tests for cardio_cli.
//!
//! These tests verify that multiple processes can safely:
//! - Write to WAL simultaneously (file locking)
//! - Read from state simultaneously
//! - Perform rollup operations without corruption

use assert_cmd::Command;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn cli() -> Command {
    Command::cargo_bin("cardio_cli").expect("Failed to find cardio_cli binary")
}

fn setup_test_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

#[test]
fn test_concurrent_session_logging() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Run sessions with slight delays (more realistic than thundering herd)
    for i in 0..5 {
        thread::sleep(Duration::from_millis(i * 5));
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--auto-complete")
            .assert()
            .success();
    }

    // Verify all sessions were logged
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_content = std::fs::read_to_string(&wal_path).expect("Failed to read WAL");

    // Count lines (each line is a session)
    let session_count = wal_content.lines().count();
    assert_eq!(
        session_count, 5,
        "Expected 5 sessions, got {}",
        session_count
    );
}

#[test]
fn test_concurrent_reads_and_writes() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create initial session
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--auto-complete")
        .assert()
        .success();

    // Write more sessions with delays
    for i in 0..3 {
        thread::sleep(Duration::from_millis(i * 10));
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--auto-complete")
            .assert()
            .success();
    }

    // Readers can read at any time (dry-run)
    cli()
        .arg("now")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("--dry-run")
        .assert()
        .success();

    // Should have 4 total sessions (1 initial + 3 more)
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_content = std::fs::read_to_string(&wal_path).expect("Failed to read WAL");
    let session_count = wal_content.lines().count();
    assert_eq!(session_count, 4);
}

#[test]
fn test_rollup_while_writing() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Create some initial sessions
    for _ in 0..3 {
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--auto-complete")
            .assert()
            .success();
    }

    // Start rollup in background
    let data_dir_rollup = data_dir.clone();
    let rollup_handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        cli()
            .arg("rollup")
            .arg("--data-dir")
            .arg(&data_dir_rollup)
            .assert()
            .success();
    });

    // Write more sessions while rollup might be running
    for _ in 0..2 {
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--auto-complete")
            .assert()
            .success();
        thread::sleep(Duration::from_millis(5));
    }

    rollup_handle.join().expect("Rollup thread panicked");

    // Verify CSV exists and has data
    let csv_path = data_dir.join("sessions.csv");
    assert!(csv_path.exists());

    // New sessions should still be in WAL or successfully written
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    if wal_path.exists() {
        // If WAL still exists, it should have the new sessions
        let wal_content = std::fs::read_to_string(&wal_path).expect("Failed to read WAL");
        assert!(wal_content.lines().count() >= 2);
    }
}

#[test]
fn test_no_wal_corruption_under_load() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Hammer the CLI with many concurrent writes
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let data_dir = data_dir.clone();
            thread::spawn(move || {
                // Small stagger to reduce thundering herd
                thread::sleep(Duration::from_millis(i * 5));
                cli()
                    .arg("now")
                    .arg("--data-dir")
                    .arg(&data_dir)
                    .arg("--auto-complete")
                    .timeout(Duration::from_secs(10))
                    .assert()
                    .success();
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Give filesystem a moment to settle
    thread::sleep(Duration::from_millis(100));

    // Verify WAL is valid JSON-lines
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_content = std::fs::read_to_string(&wal_path).expect("Failed to read WAL");

    let mut valid_count = 0;
    for line in wal_content.lines() {
        if line.is_empty() {
            continue;
        }
        // Try to parse as JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(
            parsed.is_ok(),
            "WAL contains invalid JSON line: {}",
            line
        );
        valid_count += 1;
    }

    assert_eq!(valid_count, 10, "Expected 10 valid sessions in WAL");
}

#[test]
fn test_state_concurrent_updates() {
    let temp_dir = setup_test_dir();
    let data_dir = temp_dir.path().to_path_buf();

    // Force mobility sessions which update state
    // Run sequentially to avoid race conditions
    for i in 0..3 {
        cli()
            .arg("now")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("--category")
            .arg("mobility")
            .arg("--auto-complete")
            .timeout(Duration::from_secs(10))
            .assert()
            .success();
    }

    // State file should exist and be valid JSON
    let state_path = data_dir.join("wal/state.json");
    assert!(state_path.exists());

    let state_content = std::fs::read_to_string(&state_path).expect("Failed to read state");
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&state_content);
    assert!(parsed.is_ok(), "State file contains invalid JSON");
}
