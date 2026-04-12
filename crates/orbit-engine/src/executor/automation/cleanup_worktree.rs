use std::path::{Path, PathBuf};

use orbit_types::OrbitError;
use serde_json::{Map, Value, json};

use crate::context::RuntimeHost;

use super::git::{
    git_command_success, git_output, git_output_raw, resolve_worktree_path_from_prefix,
};
use super::input::input_string_field;

const DEFAULT_BRANCH_PREFIX: &str = "orbit";

pub(super) fn cleanup_worktree<H: RuntimeHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let run_id = super::parallel::require_run_id(input, "cleanup_worktree")?;
    let repo_root_str = host.repo_root()?;
    let repo_root = Path::new(&repo_root_str);
    let workspace_path = resolve_workspace_path(repo_root, input, run_id)?;
    let workspace_path_str = workspace_path.to_string_lossy().to_string();
    let branch_name = detect_branch_name(repo_root, &workspace_path);

    let _ = git_command_success(
        repo_root,
        &["worktree", "remove", "--force", workspace_path_str.as_str()],
    );
    let _ = git_command_success(repo_root, &["worktree", "prune"]);
    if let Some(branch_name) = branch_name.as_deref() {
        let _ = git_command_success(repo_root, &["branch", "-D", branch_name]);
    }

    let mut output = Map::new();
    output.insert("cleaned_up".to_string(), json!(true));
    output.insert("workspace_path".to_string(), json!(workspace_path_str));
    if let Some(branch_name) = branch_name {
        output.insert("branch".to_string(), json!(branch_name));
    }
    Ok(Value::Object(output))
}

fn resolve_workspace_path(
    repo_root: &Path,
    input: &Value,
    run_id: &str,
) -> Result<PathBuf, OrbitError> {
    if let Some(workspace_path) = input_string_field(input, "workspace_path") {
        return Ok(absolute_workspace_path(repo_root, &workspace_path));
    }

    if let Some(branch_prefix) = input_string_field(input, "branch_prefix") {
        return resolve_worktree_path_from_prefix(repo_root, &branch_prefix, run_id);
    }

    if has_task_id(input) {
        return resolve_worktree_path_from_prefix(repo_root, DEFAULT_BRANCH_PREFIX, run_id);
    }

    super::parallel::resolve_shared_worktree_path(repo_root, run_id)
}

fn absolute_workspace_path(repo_root: &Path, workspace_path: &str) -> PathBuf {
    let workspace_path = PathBuf::from(workspace_path);
    if workspace_path.is_absolute() {
        workspace_path
    } else {
        repo_root.join(workspace_path)
    }
}

fn has_task_id(input: &Value) -> bool {
    input
        .get("task_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|task_id| !task_id.is_empty())
}

fn detect_branch_name(repo_root: &Path, workspace_path: &Path) -> Option<String> {
    if workspace_path.is_dir()
        && let Ok(branch_name) = git_output(workspace_path, &["rev-parse", "--abbrev-ref", "HEAD"])
    {
        let branch_name = branch_name.trim();
        if !branch_name.is_empty() && branch_name != "HEAD" {
            return Some(branch_name.to_string());
        }
    }

    let worktree_list = git_output_raw(repo_root, &["worktree", "list", "--porcelain"]).ok()?;
    branch_name_from_worktree_list(&worktree_list, workspace_path)
}

fn branch_name_from_worktree_list(worktree_list: &str, workspace_path: &Path) -> Option<String> {
    let target_path = workspace_path.to_string_lossy();
    let mut matching_block = false;

    for line in worktree_list.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            matching_block = path == target_path;
            continue;
        }

        if !matching_block {
            continue;
        }

        if let Some(branch_name) = line.strip_prefix("branch refs/heads/") {
            return Some(branch_name.to_string());
        }

        if line.is_empty() {
            matching_block = false;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::cleanup_worktree;
    use crate::context::{JobRunResult, RuntimeHost};
    use orbit_tools::ToolContext;
    use orbit_types::{
        Activity, InvocationTrace, Job, JobTargetType, OrbitError, OrbitEvent, Role,
    };

    struct MockRuntimeHost {
        repo_root: PathBuf,
        data_root: TempDir,
    }

    impl RuntimeHost for MockRuntimeHost {
        fn record_event(&self, _event: OrbitEvent) -> Result<(), OrbitError> {
            unreachable!("not used in test")
        }

        fn repo_root(&self) -> Result<String, OrbitError> {
            Ok(self.repo_root.to_string_lossy().to_string())
        }

        fn data_root(&self) -> &Path {
            self.data_root.path()
        }

        fn run_job_now_with_input_debug(
            &self,
            _job_id: &str,
            _input: Value,
            _debug: bool,
        ) -> Result<JobRunResult, OrbitError> {
            unreachable!("not used in test")
        }

        fn validate_activity_target_exists(
            &self,
            _target_type: JobTargetType,
            _target_id: &str,
        ) -> Result<Activity, OrbitError> {
            unreachable!("not used in test")
        }

        fn get_job(&self, _job_id: &str) -> Result<Option<Job>, OrbitError> {
            unreachable!("not used in test")
        }

        fn run_tool_with_context_and_role(
            &self,
            _name: &str,
            _input: Value,
            _role: Role,
            _tool_context: ToolContext,
        ) -> Result<Value, OrbitError> {
            unreachable!("not used in test")
        }

        fn maybe_create_failure_task(
            &self,
            _job_id: &str,
            _run_id: &str,
            _error_code: &str,
            _error_message: &str,
            _agent: Option<&str>,
            _model: Option<&str>,
        ) -> Result<(), OrbitError> {
            unreachable!("not used in test")
        }

        fn scoring_enabled(&self) -> bool {
            false
        }

        fn graph_editing(&self) -> bool {
            false
        }

        fn scoreboard_dir(&self) -> &Path {
            self.data_root.path()
        }

        fn persist_invocation_trace(
            &self,
            _job_run_id: &str,
            _execution: &crate::context::ExecutionContext,
            _trace: &InvocationTrace,
        ) -> Result<(), OrbitError> {
            Ok(())
        }
    }

    #[test]
    fn cleanup_worktree_removes_directory_and_branch_after_successful_merge() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let worktree = temp.path().join("worktree");
        let branch_name = "duel/T-cleanup-1234";
        init_repo(&repo_root);
        add_worktree(&repo_root, &worktree, branch_name);

        let host = MockRuntimeHost {
            repo_root: repo_root.clone(),
            data_root: tempfile::tempdir().expect("data root"),
        };

        let output = cleanup_worktree(
            &host,
            &json!({
                "run_id": "jrun-1",
                "workspace_path": worktree.to_string_lossy().to_string(),
            }),
        )
        .expect("cleanup succeeds");

        assert_eq!(output["cleaned_up"], json!(true));
        assert_eq!(output["workspace_path"], json!(worktree.to_string_lossy()));
        assert_eq!(output["branch"], json!(branch_name));
        assert!(!worktree.exists(), "worktree should be removed");
        assert_eq!(
            git_stdout(&repo_root, &["branch", "--list", branch_name]),
            ""
        );
        assert!(
            !git_stdout(&repo_root, &["worktree", "list", "--porcelain"])
                .contains(worktree.to_string_lossy().as_ref()),
            "removed worktree should not remain registered"
        );
    }

    #[test]
    fn cleanup_worktree_succeeds_even_when_worktree_already_gone() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        init_repo(&repo_root);

        let host = MockRuntimeHost {
            repo_root: repo_root.clone(),
            data_root: tempfile::tempdir().expect("data root"),
        };

        let missing_worktree = repo_root
            .join(".orbit")
            .join("worktrees")
            .join("duel-jrun-2");
        let output = cleanup_worktree(
            &host,
            &json!({
                "run_id": "jrun-2",
                "workspace_path": missing_worktree.to_string_lossy().to_string(),
            }),
        )
        .expect("cleanup succeeds");

        assert_eq!(output["cleaned_up"], json!(true));
        assert_eq!(
            output["workspace_path"],
            json!(missing_worktree.to_string_lossy())
        );
        assert!(
            output.get("branch").is_none(),
            "branch should be omitted when it cannot be resolved"
        );
    }

    fn init_repo(repo_root: &Path) {
        std::fs::create_dir_all(repo_root).expect("create repo");
        run_git(repo_root, &["init", "-b", "main"]);
        run_git(repo_root, &["config", "user.name", "Orbit Tests"]);
        run_git(
            repo_root,
            &["config", "user.email", "orbit-tests@example.com"],
        );
        run_git(repo_root, &["config", "commit.gpgsign", "false"]);
        std::fs::write(repo_root.join("tracked.txt"), "base\n").expect("write tracked.txt");
        run_git(repo_root, &["add", "tracked.txt"]);
        run_git(repo_root, &["commit", "-m", "initial"]);
    }

    fn add_worktree(repo_root: &Path, worktree: &Path, branch: &str) {
        let worktree_str = worktree.to_string_lossy().into_owned();
        run_git(
            repo_root,
            &[
                "worktree",
                "add",
                "-b",
                branch,
                worktree_str.as_str(),
                "main",
            ],
        );
    }

    fn run_git(current_dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(current_dir)
            .args(args)
            .output()
            .expect("git output");
        assert!(
            output.status.success(),
            "git {} failed in '{}': stdout={} stderr={}",
            args.join(" "),
            current_dir.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(current_dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(current_dir)
            .args(args)
            .output()
            .expect("git output");
        assert!(
            output.status.success(),
            "git {} failed in '{}': stdout={} stderr={}",
            args.join(" "),
            current_dir.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("utf8")
            .trim()
            .to_string()
    }
}
