use std::fs;
use std::path::PathBuf;
use std::process::Output;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::{TempDir, tempdir};

#[test]
fn task_tools_reject_dropped_task_types_with_valid_options() {
    let workspace = TestWorkspace::new();

    for dropped_type in ["epic", "task", "issue"] {
        let output = workspace.run_raw(
            &[
                "tool",
                "run",
                "orbit.task.add",
                "--input",
                &format!(
                    r#"{{"title":"Dropped type","description":"Rejected.","workspace":".","type":"{dropped_type}"}}"#
                ),
            ],
            "reject dropped task type on add",
        );
        assert!(!output.status.success());
        let combined = output_text(&output);
        assert!(combined.contains(dropped_type), "output:\n{combined}");
        assert!(
            combined.contains("feature, bug, refactor, chore"),
            "output:\n{combined}"
        );
    }

    let created = workspace.run(
        &[
            "tool",
            "run",
            "orbit.task.add",
            "--full",
            "--input",
            r#"{"title":"Modern type","description":"Created.","workspace":".","type":"feature"}"#,
        ],
        "create task",
    );
    let task: Value = serde_json::from_slice(&created.stdout).expect("task JSON");
    let task_id = task["id"].as_str().expect("task id");
    let output = workspace.run_raw(
        &[
            "tool",
            "run",
            "orbit.task.update",
            "--input",
            &format!(r#"{{"id":"{task_id}","type":"issue"}}"#),
        ],
        "reject dropped task type on update",
    );
    assert!(!output.status.success());
    let combined = output_text(&output);
    assert!(combined.contains("issue"), "output:\n{combined}");
    assert!(
        combined.contains("feature, bug, refactor, chore"),
        "output:\n{combined}"
    );
}

#[test]
fn migrate_task_types_dry_run_write_and_second_run_are_idempotent() {
    let workspace = TestWorkspace::new();
    let task = workspace.write_task("proposed", "T20260510-1", "task");
    let epic = workspace.write_task("backlog", "T20260510-2", "epic");
    let issue = workspace.write_task("review", "T20260510-3", "issue");
    let friction = workspace.write_task("friction", "T20260510-4", "friction");
    let friction_before = fs::read_to_string(&friction).expect("read friction");

    let dry_run = workspace.run(
        &["migrate", "task-types", "--dry-run"],
        "dry-run task type migration",
    );
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("T20260510-1: task -> chore"));
    assert!(dry_run_stdout.contains("T20260510-2: epic -> feature"));
    assert!(dry_run_stdout.contains("T20260510-3: issue -> bug"));
    assert!(dry_run_stdout.contains("would migrate task types: 3 changed"));
    assert!(
        fs::read_to_string(&task)
            .expect("task")
            .contains("type: task")
    );

    let migrated = workspace.run(&["migrate", "task-types"], "write task type migration");
    assert!(
        String::from_utf8_lossy(&migrated.stdout).contains("migrated task types: 3 changed"),
        "stdout:\n{}",
        String::from_utf8_lossy(&migrated.stdout)
    );
    assert!(
        fs::read_to_string(task)
            .expect("task")
            .contains("type: chore")
    );
    assert!(
        fs::read_to_string(epic)
            .expect("epic")
            .contains("type: feature")
    );
    assert!(
        fs::read_to_string(issue)
            .expect("issue")
            .contains("type: bug")
    );
    assert_eq!(
        fs::read_to_string(friction).expect("friction"),
        friction_before
    );

    let second = workspace.run(&["migrate", "task-types"], "second task type migration");
    assert!(
        String::from_utf8_lossy(&second.stdout).contains("migrated task types: 0 changed"),
        "stdout:\n{}",
        String::from_utf8_lossy(&second.stdout)
    );
}

struct TestWorkspace {
    _temp: TempDir,
    home: PathBuf,
    work: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let temp = tempdir().expect("tempdir");
        let home = temp.path().join("home");
        let work = temp.path().join("work");
        fs::create_dir_all(&home).expect("create home");
        fs::create_dir_all(&work).expect("create work");

        let workspace = Self {
            _temp: temp,
            home,
            work,
        };
        workspace.run(
            &["workspace", "init", "--name", "task-type-test"],
            "init workspace",
        );
        workspace
    }

    fn write_task(&self, state: &str, id: &str, task_type: &str) -> PathBuf {
        let dir = self.work.join(".orbit/tasks").join(state).join(id);
        fs::create_dir_all(&dir).expect("create task dir");
        fs::write(dir.join("plan.md"), "").expect("write plan");
        fs::write(dir.join("execution-summary.md"), "").expect("write summary");
        let path = dir.join("task.yaml");
        fs::write(
            &path,
            format!(
                "schema_version: 2\nid: {id}\ntype: {task_type}\npriority: medium\ntitle: {id}\ndescription: Migration fixture.\nacceptance_criteria: []\ndependencies: []\ncontext_files: []\nworkspace_path: .\nrepo_root: null\ncreated_at: 2026-05-10T00:00:00Z\nupdated_at: 2026-05-10T00:00:00Z\nhistory: []\ncomments: []\n"
            ),
        )
        .expect("write task yaml");
        path
    }

    fn run(&self, args: &[&str], label: &str) -> Output {
        let output = self.run_raw(args, label);
        assert!(
            output.status.success(),
            "{label} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn run_raw(&self, args: &[&str], _label: &str) -> Output {
        let mut command = cargo_bin_cmd!("orbit");
        command
            .current_dir(&self.work)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove("ORBIT_ROOT")
            .args(args);
        command.output().expect("run orbit")
    }
}

fn output_text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
