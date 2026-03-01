use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("HOME", dir);
    cmd.env("USERPROFILE", dir);
    cmd
}

fn add_task(dir: &Path, title: &str) -> String {
    let output = orbit_in(dir)
        .args(["task", "add", "--title", title])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("utf8").trim().to_string()
}

#[test]
fn task_add_prints_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    let id = add_task(dir.path(), "test task");
    assert!(id.starts_with("T"), "id should start with T: {id}");
}

#[test]
fn task_list_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID"));
}

#[test]
fn task_list_after_add() {
    let dir = tempfile::tempdir().expect("tempdir");
    let id = add_task(dir.path(), "listed task");
    orbit_in(dir.path())
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&id))
        .stdout(predicate::str::contains("listed task"));
}

#[test]
fn task_list_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let _id = add_task(dir.path(), "json task");
    let output = orbit_in(dir.path())
        .args(["task", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let parsed: serde_json::Value = serde_json::from_slice(&output).expect("valid JSON");
    assert!(parsed.is_array());
    let arr = parsed.as_array().expect("array");
    assert!(arr.iter().any(|t| t["title"] == "json task"));
}

#[test]
fn task_show_displays_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    let id = add_task(dir.path(), "showable task");
    orbit_in(dir.path())
        .args(["task", "show", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID:"))
        .stdout(predicate::str::contains("showable task"))
        .stdout(predicate::str::contains("todo"));
}

#[test]
fn task_show_nonexistent() {
    let dir = tempfile::tempdir().expect("tempdir");
    orbit_in(dir.path())
        .args(["task", "show", "task-nonexistent"])
        .assert()
        .failure();
}

#[test]
fn task_update_changes_title() {
    let dir = tempfile::tempdir().expect("tempdir");
    let id = add_task(dir.path(), "before update");
    orbit_in(dir.path())
        .args(["task", "update", &id, "--title", "after update"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated"));

    orbit_in(dir.path())
        .args(["task", "show", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("after update"));
}

#[test]
fn task_close_and_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let id = add_task(dir.path(), "closable");
    orbit_in(dir.path())
        .args(["task", "close", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed"));

    orbit_in(dir.path())
        .args(["task", "show", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    orbit_in(dir.path())
        .args(["task", "reopen", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reopened"));

    orbit_in(dir.path())
        .args(["task", "show", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("todo"));
}

#[test]
fn task_delete_removes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let id = add_task(dir.path(), "deletable");
    orbit_in(dir.path())
        .args(["task", "delete", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted"));

    orbit_in(dir.path())
        .args(["task", "show", &id])
        .assert()
        .failure();
}

#[test]
fn task_search_matches() {
    let dir = tempfile::tempdir().expect("tempdir");
    let _id = add_task(dir.path(), "unique-searchable-xyz");
    orbit_in(dir.path())
        .args(["task", "search", "unique-searchable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unique-searchable-xyz"));
}

#[test]
fn task_workspace_add_update_and_clear() {
    let dir = tempfile::tempdir().expect("tempdir");
    let workspace = dir.path().join("repo");
    std::fs::create_dir_all(&workspace).expect("workspace dir");
    let workspace_canonical = workspace.canonicalize().expect("canonical workspace");

    let output = orbit_in(dir.path())
        .args([
            "task",
            "add",
            "--title",
            "workspace task",
            "--workspace",
            workspace.to_string_lossy().as_ref(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let id = String::from_utf8(output).expect("utf8").trim().to_string();

    let show_output = orbit_in(dir.path())
        .args(["task", "show", &id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show: serde_json::Value = serde_json::from_slice(&show_output).expect("show json");
    assert_eq!(
        show["workspace_path"],
        workspace_canonical.to_string_lossy().to_string()
    );

    orbit_in(dir.path())
        .args(["task", "update", &id, "--workspace", ""])
        .assert()
        .success();

    let show_after_clear = orbit_in(dir.path())
        .args(["task", "show", &id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show_after_clear: serde_json::Value =
        serde_json::from_slice(&show_after_clear).expect("show json after clear");
    assert!(show_after_clear["workspace_path"].is_null());
}

#[test]
fn task_approve_sets_approval_fields() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".orbit")).expect("create .orbit");
    std::fs::write(
        dir.path().join(".orbit").join("config.toml"),
        "[task.approval]\nrequired_for_agent = true\n",
    )
    .expect("write config");
    let id = add_task(dir.path(), "approvable");

    orbit_in(dir.path())
        .args([
            "task",
            "approve",
            &id,
            "--by",
            "daniel",
            "--note",
            "approved verbally in sync",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Approved task"));

    let show_output = orbit_in(dir.path())
        .args(["task", "show", &id, "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let show: serde_json::Value = serde_json::from_slice(&show_output).expect("show json");
    assert_eq!(show["approved_by"], "daniel");
    assert_eq!(show["approval_note"], "approved verbally in sync");
    assert!(show["approved_at"].is_string());
}
