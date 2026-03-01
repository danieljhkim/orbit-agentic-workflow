use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("HOME", dir);
    cmd.env("USERPROFILE", dir);
    cmd
}

fn add_work(dir: &Path, id: &str) -> String {
    let output = orbit_in(dir)
        .args([
            "work",
            "add",
            "--id",
            id,
            "--type",
            "analysis",
            "--description",
            "test spec",
            "--input-schema",
            "{}",
            "--output-schema",
            "{}",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("utf8").trim().to_string()
}

fn add_job(dir: &Path, target_id: &str, schedule: &str, agent_cli: &str) -> String {
    let output = orbit_in(dir)
        .args([
            "job",
            "add",
            "--target-id",
            target_id,
            "--schedule",
            schedule,
            "--agent-cli",
            agent_cli,
            "--timeout",
            "30s",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("utf8").trim().to_string()
}

fn write_mock_agent(dir: &Path) -> String {
    let path = dir.join("mock-agent");
    std::fs::write(
        &path,
        "#!/bin/sh\nprintf '{\"schemaVersion\":1,\"status\":\"success\",\"result\":{},\"error\":null,\"durationMs\":1}'\n",
    )
    .expect("write mock agent");
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod mock agent");
    path.to_string_lossy().to_string()
}

fn write_failing_agent(dir: &Path) -> String {
    let path = dir.join("mock-agent");
    std::fs::write(&path, "#!/bin/sh\necho 'network down' 1>&2\nexit 1\n")
        .expect("write failing agent");
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod failing agent");
    path.to_string_lossy().to_string()
}

#[test]
fn job_add_list_show_json_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_id = add_work(dir.path(), "spec-cli-list");

    let job_id = add_job(dir.path(), &spec_id, "every 1m", "mock-agent");
    assert!(job_id.starts_with("job-"), "unexpected job id: {job_id}");

    let list_output = orbit_in(dir.path())
        .args(["job", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: Value = serde_json::from_slice(&list_output).expect("list json");
    let arr = list.as_array().expect("array");
    assert!(arr.iter().any(|job| job["job_id"] == job_id));

    let show_output = orbit_in(dir.path())
        .args(["job", "show", &job_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show: Value = serde_json::from_slice(&show_output).expect("show json");
    assert_eq!(show["job_id"], job_id);
    assert_eq!(show["target_type"], "work");
    assert_eq!(show["target_id"], spec_id);
    assert_eq!(show["schedule"], "every 1m");
    assert_eq!(show["state"], "enabled");
}

#[test]
fn job_run_creates_run_and_history_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_id = add_work(dir.path(), "spec-cli-run");
    let agent_cli = write_mock_agent(dir.path());
    let job_id = add_job(dir.path(), &spec_id, "every 1m", &agent_cli);

    let run_output = orbit_in(dir.path())
        .args(["job", "run", &job_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let run: Value = serde_json::from_slice(&run_output).expect("run json");
    assert_eq!(run["job_id"], job_id);
    assert_eq!(run["state"], "success");
    assert_eq!(run["attempt"], 1);

    let history_output = orbit_in(dir.path())
        .args(["job", "history", &job_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let history: Value = serde_json::from_slice(&history_output).expect("history json");
    let runs = history.as_array().expect("array");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["state"], "success");
    assert_eq!(runs[0]["attempt"], 1);
    assert!(runs[0]["agent_response_json"].is_object());
}

#[test]
fn job_pause_resume_delete_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_id = add_work(dir.path(), "spec-cli-state");
    let job_id = add_job(dir.path(), &spec_id, "every 1m", "mock-agent");

    orbit_in(dir.path())
        .args(["job", "pause", &job_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Paused job"));

    let paused_output = orbit_in(dir.path())
        .args(["job", "show", &job_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let paused: Value = serde_json::from_slice(&paused_output).expect("paused json");
    assert_eq!(paused["state"], "paused");

    orbit_in(dir.path())
        .args(["job", "resume", &job_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Resumed job"));

    let resumed_output = orbit_in(dir.path())
        .args(["job", "show", &job_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let resumed: Value = serde_json::from_slice(&resumed_output).expect("resumed json");
    assert_eq!(resumed["state"], "enabled");

    orbit_in(dir.path())
        .args(["job", "delete", &job_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted job"));

    let list_output = orbit_in(dir.path())
        .args(["job", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list: Value = serde_json::from_slice(&list_output).expect("list json");
    let arr = list.as_array().expect("array");
    assert!(!arr.iter().any(|job| job["job_id"] == job_id));
}

#[test]
fn job_run_failure_json_includes_error_details() {
    let dir = tempfile::tempdir().expect("tempdir");
    let spec_id = add_work(dir.path(), "spec-cli-run-fail");
    let agent_cli = write_failing_agent(dir.path());
    let job_id = add_job(dir.path(), &spec_id, "every 1m", &agent_cli);

    let run_output = orbit_in(dir.path())
        .args(["job", "run", &job_id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let run: Value = serde_json::from_slice(&run_output).expect("run json");
    assert_eq!(run["job_id"], job_id);
    assert_eq!(run["state"], "failed");
    assert_eq!(run["error_code"], "AGENT_INVOCATION_FAILED");
    assert!(
        run["error_message"]
            .as_str()
            .unwrap_or_default()
            .contains("network down")
    );
}

#[test]
fn job_add_rejects_workflow_target_type() {
    let dir = tempfile::tempdir().expect("tempdir");

    orbit_in(dir.path())
        .args([
            "job",
            "add",
            "--target-type",
            "workflow",
            "--target-id",
            "wf-1",
            "--schedule",
            "every 1m",
            "--agent-cli",
            "mock-agent",
            "--timeout",
            "30s",
        ])
        .assert()
        .failure();
}
