use assert_cmd::Command;
use predicates::prelude::*;

fn orbit() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("orbit").expect("binary exists")
}

#[test]
fn tool_list_exits_zero_and_shows_builtins() {
    orbit()
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
    let output = orbit()
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
    orbit()
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
    orbit()
        .args(["tool", "show", "nonexistent.tool"])
        .assert()
        .failure();
}

#[test]
fn tool_run_with_input_json() {
    let output = orbit()
        .args([
            "tool",
            "run",
            "fs.read",
            "--input",
            r#"{"path":"Cargo.toml"}"#,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value =
        serde_json::from_slice(&output).expect("output should be valid JSON");
    assert!(parsed["content"].is_string());
    assert_eq!(parsed["path"], "Cargo.toml");
}

#[test]
fn tool_run_dry_run_does_not_execute() {
    orbit()
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
    orbit()
        .args(["tool", "run", "nonexistent.tool"])
        .assert()
        .failure();
}

#[test]
fn tool_doctor_exits_zero_for_healthy() {
    orbit()
        .args(["tool", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("All tools healthy"));
}
