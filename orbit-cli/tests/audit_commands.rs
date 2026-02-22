use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::Path;

fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd
}

#[test]
fn audit_list_runs_successfully() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["audit", "list"])
        .assert()
        .success();
}

#[test]
fn audit_list_json_returns_empty_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = orbit_in(dir.path())
        .args(["audit", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: Value = serde_json::from_slice(&output).expect("json");
    assert!(list.as_array().expect("array").is_empty());
}

#[test]
fn audit_show_missing_returns_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["audit", "show", "99999"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("audit event not found"));
}

#[test]
fn audit_prune_runs_successfully() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["audit", "prune", "--older-than", "90d"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pruned"));
}

#[test]
fn audit_stats_runs_successfully() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["audit", "stats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Total:"));
}

#[test]
fn audit_stats_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = orbit_in(dir.path())
        .args(["audit", "stats", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stats: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(stats["total"], 0);
}

#[test]
fn audit_export_json_creates_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_file = dir.path().join("audit.json");
    orbit_in(dir.path())
        .args([
            "audit",
            "export",
            "--format",
            "json",
            "--output",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Exported"));

    let content = std::fs::read_to_string(&output_file).expect("read file");
    let parsed: Value = serde_json::from_str(&content).expect("valid json");
    assert!(parsed.as_array().expect("array").is_empty());
}

#[test]
fn audit_export_csv_creates_file_with_header() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_file = dir.path().join("audit.csv");
    orbit_in(dir.path())
        .args([
            "audit",
            "export",
            "--format",
            "csv",
            "--output",
            output_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Exported"));

    let content = std::fs::read_to_string(&output_file).expect("read file");
    assert!(content.starts_with("id,execution_id,timestamp,"));
}

#[test]
fn middleware_records_audit_event_for_tool_list() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Run a tool list command, which should be audited
    orbit_in(dir.path())
        .args(["tool", "list"])
        .assert()
        .success();

    // Now check that an audit event was recorded
    let output = orbit_in(dir.path())
        .args(["audit", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: Value = serde_json::from_slice(&output).expect("json");
    let events = list.as_array().expect("array");
    assert!(
        events.iter().any(|e| e["command"] == "tool"),
        "expected audit event for tool command"
    );
}

#[test]
fn audit_commands_are_not_audited() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Run an audit command
    orbit_in(dir.path())
        .args(["audit", "list"])
        .assert()
        .success();

    // Audit list should show zero events (audit commands skip middleware)
    let output = orbit_in(dir.path())
        .args(["audit", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: Value = serde_json::from_slice(&output).expect("json");
    let events = list.as_array().expect("array");
    assert!(
        !events.iter().any(|e| e["command"] == "audit"),
        "audit commands should not be recorded"
    );
}
