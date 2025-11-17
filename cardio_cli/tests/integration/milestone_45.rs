use assert_cmd::prelude::*;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use tempfile::TempDir;

fn bin_path() -> PathBuf {
    assert_cmd::cargo::cargo_bin!("krep").to_path_buf()
}

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("temp dir")
}

fn read_wal_lines(path: &Path) -> Vec<String> {
    let content = fs::read_to_string(path).expect("read wal");
    content
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn clean_environment_full_cycle() {
    let temp_dir = temp_dir();
    let data_dir = temp_dir.path();

    // First run auto-completes a real session
    Command::new(bin_path())
        .arg("now")
        .arg("--auto-complete")
        .arg("--data-dir")
        .arg(data_dir)
        .assert()
        .success();

    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_lines = read_wal_lines(&wal_path);
    assert_eq!(
        wal_lines.len(),
        1,
        "expected exactly one real session in WAL"
    );

    // State file should exist and have at least one progression entry
    let state_path = data_dir.join("wal/state.json");
    let state: Value = serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let progressions = state
        .get("progressions")
        .and_then(|v| v.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);
    assert!(progressions, "state.json should contain progression data");

    // Rollup and cleanup should archive then remove processed WAL
    Command::new(bin_path())
        .arg("rollup")
        .arg("--cleanup")
        .arg("--data-dir")
        .arg(data_dir)
        .assert()
        .success();

    assert!(
        !wal_path.exists(),
        "wal should be removed or archived after rollup"
    );
    assert!(
        !data_dir
            .join("wal/microdose_sessions.wal.processed")
            .exists(),
        "processed WAL files should be cleaned up"
    );

    let csv_path = data_dir.join("sessions.csv");
    let csv_content = fs::read_to_string(&csv_path).unwrap();
    assert!(
        csv_content.lines().any(|l| l.contains("definition_id")),
        "CSV should contain session rows"
    );
}

#[test]
fn skip_rotates_categories() {
    let temp_dir = temp_dir();
    let data_dir = temp_dir.path();

    // Ensure we start on VO2 (dry-run)
    let mut saw_vo2 = false;
    for _ in 0..3 {
        let output = Command::new(bin_path())
            .arg("now")
            .arg("--dry-run")
            .arg("--data-dir")
            .arg(data_dir)
            .output()
            .expect("dry-run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.to_lowercase().contains("vo2 microdose") {
            saw_vo2 = true;
            break;
        }
    }
    assert!(saw_vo2, "engine should offer a VO2 prescription");

    // Auto-skip should cycle through categories before completing
    let output = Command::new(bin_path())
        .arg("now")
        .arg("--auto-complete-skip")
        .arg("--data-dir")
        .arg(data_dir)
        .output()
        .expect("auto-skip");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sequence = Vec::new();
    for line in stdout.lines() {
        if line.contains("MICRODOSE") {
            let l = line.to_lowercase();
            if l.contains("vo2") {
                sequence.push("vo2");
            } else if l.contains("gtg") {
                sequence.push("gtg");
            } else if l.contains("mobility") {
                sequence.push("mobility");
            }
        }
    }

    assert!(
        sequence.starts_with(&["vo2", "gtg", "mobility", "vo2"]),
        "expected rotation Vo2 → Gtg → Mobility → Vo2, got {:?}",
        sequence
    );
}

#[test]
fn strength_override_prefers_gtg() {
    let temp_dir = temp_dir();
    let data_dir = temp_dir.path();
    let strength_dir = data_dir.join("strength");
    fs::create_dir_all(&strength_dir).unwrap();

    // Write a recent lower-body strength signal
    let now = chrono::Utc::now().to_rfc3339();
    let signal = format!(r#"{{"last_session_at":"{}","session_type":"lower"}}"#, now);
    fs::write(strength_dir.join("signal.json"), signal).unwrap();

    let output = Command::new(bin_path())
        .arg("now")
        .arg("--auto-complete")
        .arg("--data-dir")
        .arg(data_dir)
        .output()
        .expect("run with strength signal");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Gtg MICRODOSE") || stdout.contains("GTG MICRODOSE"),
        "strength override should prescribe GTG, got stdout: {stdout}"
    );

    // WAL should contain a GTG pull-up session
    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_lines = read_wal_lines(&wal_path);
    let parsed: Value = serde_json::from_str(&wal_lines[0]).unwrap();
    assert!(
        parsed["definition_id"].as_str().unwrap().contains("gtg"),
        "expected GTG definition in WAL"
    );
}

#[test]
fn partial_wal_corruption_is_salvageable() {
    let temp_dir = temp_dir();
    let data_dir = temp_dir.path();

    // Create a couple of valid sessions
    for _ in 0..2 {
        Command::new(bin_path())
            .arg("now")
            .arg("--auto-complete")
            .arg("--data-dir")
            .arg(data_dir)
            .assert()
            .success();
    }

    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let mut wal = fs::OpenOptions::new().append(true).open(&wal_path).unwrap();
    writeln!(wal, "{{garbage line").unwrap();

    // Should still load and append without panic
    Command::new(bin_path())
        .arg("now")
        .arg("--auto-complete")
        .arg("--data-dir")
        .arg(data_dir)
        .assert()
        .success();

    let lines = read_wal_lines(&wal_path);
    let valid_count = lines
        .iter()
        .filter(|l| serde_json::from_str::<Value>(l).is_ok())
        .count();
    assert!(
        valid_count >= 3,
        "expected valid sessions to be preserved despite corruption"
    );
}

#[test]
fn corrupted_state_file_is_recovered() {
    let temp_dir = temp_dir();
    let data_dir = temp_dir.path();

    let wal_dir = data_dir.join("wal");
    fs::create_dir_all(&wal_dir).unwrap();
    let state_path = wal_dir.join("state.json");
    fs::write(&state_path, &[0u8, 159, 146, 150]).unwrap(); // invalid UTF-8

    Command::new(bin_path())
        .arg("now")
        .arg("--auto-complete")
        .arg("--data-dir")
        .arg(data_dir)
        .assert()
        .success();

    let state_content = fs::read_to_string(&state_path).unwrap();
    serde_json::from_str::<Value>(&state_content)
        .expect("state should be valid JSON after recovery");
}

#[test]
fn concurrent_calls_do_not_corrupt_wal() {
    let temp_dir = temp_dir();
    let data_dir = temp_dir.path().to_path_buf();
    let bin = bin_path();

    let handles: Vec<_> = (0..2)
        .map(|_| {
            let bin = bin.clone();
            let data_dir = data_dir.clone();
            thread::spawn(move || {
                Command::new(&bin)
                    .arg("now")
                    .arg("--auto-complete")
                    .arg("--data-dir")
                    .arg(&data_dir)
                    .output()
                    .expect("run")
            })
        })
        .collect();

    for handle in handles {
        let output = handle.join().expect("thread");
        assert!(
            output.status.success(),
            "CLI call failed (status {:?})\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    let wal_path = data_dir.join("wal/microdose_sessions.wal");
    let wal_lines = read_wal_lines(&wal_path);
    assert_eq!(wal_lines.len(), 2, "expected two sessions in WAL");

    for line in wal_lines {
        serde_json::from_str::<Value>(&line).expect("WAL line should be valid JSON");
    }
}
