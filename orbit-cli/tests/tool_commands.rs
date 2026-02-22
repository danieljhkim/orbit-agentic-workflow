use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd
}

#[test]
fn tool_list_exits_zero_and_shows_builtins() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["tool", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("fs.read"))
        .stdout(predicate::str::contains("fs.write"))
        .stdout(predicate::str::contains("proc.spawn"))
        .stdout(predicate::str::contains("time.now"));
}

#[test]
fn tool_list_json_outputs_valid_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = orbit_in(dir.path())
        .args(["tool", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("output should be valid JSON");
    assert!(parsed.is_array());

    let tools = parsed.as_array().expect("array");
    assert!(tools.iter().any(|t| t["name"] == "fs.read"));
}

#[test]
fn tool_show_displays_parameters() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["tool", "show", "fs.read"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name:"))
        .stdout(predicate::str::contains("fs.read"))
        .stdout(predicate::str::contains("path"))
        .stdout(predicate::str::contains("required"));
}

#[test]
fn tool_show_nonexistent_exits_nonzero() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["tool", "show", "nonexistent.tool"])
        .assert()
        .failure();
}

#[test]
fn tool_run_with_input_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sample = dir.path().join("sample.txt");
    std::fs::write(&sample, "hello").expect("write sample");

    let output = orbit_in(dir.path())
        .args([
            "tool",
            "run",
            "fs.read",
            "--input",
            &format!(r#"{{"path":"{}"}}"#, sample.to_string_lossy()),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("output should be valid JSON");
    assert!(parsed["content"].is_string());
    assert_eq!(parsed["path"], sample.to_string_lossy().as_ref());
}

#[test]
fn tool_run_dry_run_does_not_execute() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args([
            "tool",
            "run",
            "fs.read",
            "--dry-run",
            "--input",
            r#"{"path":"Cargo.toml"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Policy:"))
        .stdout(predicate::str::contains("allowed"));
}

#[test]
fn tool_run_nonexistent_exits_nonzero() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["tool", "run", "nonexistent.tool"])
        .assert()
        .failure();
}

#[test]
fn tool_doctor_exits_zero_for_healthy() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["tool", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("All tools healthy"));
}
