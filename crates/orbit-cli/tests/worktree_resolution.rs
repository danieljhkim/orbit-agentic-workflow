use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use tempfile::tempdir;

#[test]
fn tool_run_task_show_resolves_main_orbit_from_git_worktree() {
    let temp = tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let main_repo = temp.path().join("repo");
    let linked_worktree = temp.path().join("repo-worktree");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&main_repo).expect("create main repo");

    run_git(&main_repo, &["init"]);
    run_git(&main_repo, &["config", "user.name", "Orbit Test"]);
    run_git(
        &main_repo,
        &["config", "user.email", "orbit-test@example.com"],
    );
    run_git(&main_repo, &["config", "commit.gpgsign", "false"]);
    fs::write(main_repo.join("README.md"), "# orbit\n").expect("write readme");
    run_git(&main_repo, &["add", "README.md"]);
    run_git(&main_repo, &["commit", "-m", "initial"]);
    run_git(
        &main_repo,
        &[
            "worktree",
            "add",
            "-b",
            "orbit-worktree-resolution",
            linked_worktree.to_str().expect("utf8 worktree path"),
        ],
    );

    let add_input = json!({
        "title": "Worktree task lookup",
        "description": "Task used by worktree resolution integration test.",
        "acceptance_criteria": ["main and linked worktree resolve the same task"],
        "workspace": ".",
        "agent": "codex",
        "model": "gpt-5"
    })
    .to_string();
    let created = run_orbit_json(
        &main_repo,
        &home,
        &[
            "tool",
            "run",
            "orbit.task.add",
            "--full",
            "--input",
            &add_input,
        ],
    );
    let task_id = created
        .get("id")
        .and_then(Value::as_str)
        .expect("created task id");

    let show_input = json!({
        "id": task_id,
        "agent": "codex",
        "model": "gpt-5"
    })
    .to_string();
    let show_args = [
        "tool",
        "run",
        "orbit.task.show",
        "--full",
        "--input",
        &show_input,
    ];
    let from_main = run_orbit_json(&main_repo, &home, &show_args);
    let from_worktree = run_orbit_json(&linked_worktree, &home, &show_args);

    assert_eq!(from_worktree, from_main);
    assert!(
        main_repo.join(".orbit").is_dir(),
        "main checkout should own the Orbit workspace"
    );
    assert!(
        !linked_worktree.join(".orbit").exists(),
        "linked worktree should not get its own Orbit workspace"
    );
}

fn run_orbit_json(cwd: &Path, home: &Path, args: &[&str]) -> Value {
    let assert = cargo_bin_cmd!("orbit")
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("ORBIT_ROOT")
        .args(args)
        .assert()
        .success();
    serde_json::from_slice(&assert.get_output().stdout).expect("orbit json output")
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git -C {} {} failed\nstdout:\n{}\nstderr:\n{}",
        cwd.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
