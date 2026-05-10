use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use tempfile::{TempDir, tempdir};

#[test]
fn task_tool_rejects_agent_and_model_only_storage_omits_agent() {
    let workspace = TestWorkspace::new();

    let rejected = workspace.run_raw(
        &[
            "tool",
            "run",
            "orbit.task.add",
            "--input",
            r#"{"title":"Legacy agent task","description":"Should fail.","workspace":".","agent":"codex"}"#,
        ],
        "reject legacy agent field",
    );
    assert!(!rejected.status.success());
    let rejected_output = format!(
        "{}{}",
        String::from_utf8_lossy(&rejected.stdout),
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(
        rejected_output.contains("use `model`"),
        "output:\n{rejected_output}"
    );

    let created = workspace.run(
        &[
            "tool",
            "run",
            "orbit.task.add",
            "--full",
            "--input",
            r#"{"title":"Model-only task","description":"Should succeed.","workspace":".","model":"gpt-5.5"}"#,
        ],
        "create model-only task",
    );
    let task: Value = serde_json::from_slice(&created.stdout).expect("created task JSON");
    let task_id = task["id"].as_str().expect("task id");
    let task_yaml = fs::read_to_string(workspace.task_yaml(task_id)).expect("read task yaml");

    assert!(task_yaml.contains("model: gpt-5.5"));
    assert!(!task_yaml.contains("agent:"));
}

#[test]
fn migration_dry_run_write_and_second_run_are_idempotent() {
    let workspace = TestWorkspace::new();
    let task_id = workspace.add_model_task("Migration task");
    let task_yaml_path = workspace.task_yaml(&task_id);
    let original = fs::read_to_string(&task_yaml_path).expect("read task yaml");
    fs::write(
        &task_yaml_path,
        original.replace("model: gpt-5.5", "agent: codex\nmodel: gpt-5.5"),
    )
    .expect("write legacy agent field");

    let dry_run = workspace.run_migration(&["--dry-run"], "dry-run migration");
    assert!(
        String::from_utf8_lossy(&dry_run.stdout).contains(&task_id),
        "stdout:\n{}",
        String::from_utf8_lossy(&dry_run.stdout)
    );
    let after_dry_run = fs::read_to_string(&task_yaml_path).expect("read task yaml");
    assert!(after_dry_run.contains("agent: codex"));

    let migrated = workspace.run_migration(&[], "write migration");
    assert!(
        String::from_utf8_lossy(&migrated.stdout).contains("normalized 1 task files"),
        "stdout:\n{}",
        String::from_utf8_lossy(&migrated.stdout)
    );
    let after_write = fs::read_to_string(&task_yaml_path).expect("read task yaml");
    assert!(!after_write.contains("agent:"));
    assert!(after_write.contains("model: gpt-5.5"));

    let second = workspace.run_migration(&[], "second migration");
    assert!(
        String::from_utf8_lossy(&second.stdout).contains("normalized 0 task files"),
        "stdout:\n{}",
        String::from_utf8_lossy(&second.stdout)
    );
}

#[test]
fn migration_fails_when_agent_has_no_model() {
    let workspace = TestWorkspace::new();
    let task_id = workspace.add_model_task("Broken migration task");
    let task_yaml_path = workspace.task_yaml(&task_id);
    let broken = fs::read_to_string(&task_yaml_path)
        .expect("read task yaml")
        .lines()
        .filter(|line| !line.starts_with("model:"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        &task_yaml_path,
        broken.replace(
            "# ---- implementation ----",
            "# ---- implementation ----\nagent: codex\n",
        ),
    )
    .expect("write broken legacy agent field");

    let output = workspace.run_migration_raw(&[], "broken migration");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&task_id), "stderr:\n{stderr}");
    assert!(
        stderr.contains("agent") && stderr.contains("model"),
        "stderr:\n{stderr}"
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
            &["workspace", "init", "--name", "task-attribution-test"],
            "init",
        );
        workspace
    }

    fn add_model_task(&self, title: &str) -> String {
        let output = self.run(
            &[
                "task",
                "add",
                "--title",
                title,
                "--description",
                "Migration fixture.",
                "--model",
                "gpt-5.5",
                "--json",
            ],
            "add model task",
        );
        let task: Value = serde_json::from_slice(&output.stdout).expect("task JSON");
        task["id"].as_str().expect("task id").to_string()
    }

    fn task_yaml(&self, task_id: &str) -> PathBuf {
        find_task_yaml(&self.work.join(".orbit").join("tasks"), task_id)
            .unwrap_or_else(|| panic!("task yaml for {task_id}"))
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

    fn run_migration(&self, args: &[&str], label: &str) -> Output {
        let output = self.run_migration_raw(args, label);
        assert!(
            output.status.success(),
            "{label} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn run_migration_raw(&self, args: &[&str], _label: &str) -> Output {
        let mut command = cargo_bin_cmd!("migrate-task-attribution");
        command
            .current_dir(&self.work)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove("ORBIT_ROOT")
            .args(args);
        command.output().expect("run migrate-task-attribution")
    }
}

fn find_task_yaml(root: &Path, task_id: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some(task_id) {
                let candidate = path.join("task.yaml");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            if let Some(found) = find_task_yaml(&path, task_id) {
                return Some(found);
            }
        }
    }
    None
}
