use std::path::Path;

use orbit_types::OrbitError;
use serde_json::{Value, json};

use crate::context::RuntimeHost;

use super::git::{
    git_command_success, git_output, git_success, refresh_local_base_branch,
    resolve_worktree_start_point,
};
use super::input::{canonicalize_existing_dir, input_string_field};

const DEFAULT_BASE: &str = "main";

pub(super) fn merge_batch_worktree_into_base<H: RuntimeHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let run_id = super::parallel::require_run_id(input, "merge_batch_worktree_into_base")?;
    let repo_root_str = host.repo_root()?;
    let repo_root = canonicalize_existing_dir(&repo_root_str, "repo_root")?;
    let workspace_path = match input_string_field(input, "workspace_path") {
        Some(path) => canonicalize_existing_dir(&path, "workspace_path")?,
        None => super::parallel::resolve_shared_worktree_path(&repo_root, run_id)?,
    };

    ensure_clean_checkout(&workspace_path, "shared batch worktree")?;

    let workspace_branch = git_output(&workspace_path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let workspace_branch = workspace_branch.trim().to_string();
    if workspace_branch == "HEAD" {
        return Err(OrbitError::Execution(
            "merge_batch_worktree_into_base: shared worktree is in detached HEAD state".to_string(),
        ));
    }

    ensure_clean_checkout(&repo_root, "base branch checkout")?;

    let base = input_string_field(input, "base").unwrap_or_else(|| DEFAULT_BASE.to_string());
    refresh_local_base_branch(&repo_root, &base);
    checkout_base_branch(&repo_root, &base)?;
    git_success(&repo_root, &["merge", "--ff-only", &workspace_branch])?;

    Ok(json!({
        "base": base,
        "workspace_path": workspace_path.to_string_lossy().to_string(),
        "workspace_branch": workspace_branch,
    }))
}

fn checkout_base_branch(repo_root: &Path, base: &str) -> Result<(), OrbitError> {
    if git_command_success(
        repo_root,
        &["rev-parse", "--verify", &format!("{base}^{{commit}}")],
    )? {
        git_success(repo_root, &["checkout", base])?;
        return Ok(());
    }

    let start_point = resolve_worktree_start_point(repo_root, base)?;
    git_success(repo_root, &["checkout", "-B", base, &start_point])?;
    Ok(())
}

fn ensure_clean_checkout(path: &Path, label: &str) -> Result<(), OrbitError> {
    let status = git_output(path, &["status", "--porcelain"])?;
    if status.trim().is_empty() {
        return Ok(());
    }

    let has_unmerged = status.lines().any(|line| {
        let bytes = line.as_bytes();
        if bytes.len() < 2 {
            return false;
        }
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D')
    });
    if has_unmerged {
        return Err(OrbitError::Execution(format!(
            "{label} '{}' has unresolved merge conflicts",
            path.display()
        )));
    }

    Err(OrbitError::Execution(format!(
        "{label} '{}' must be clean before merge_batch_worktree_into_base",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::merge_batch_worktree_into_base;
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
    fn merges_shared_worktree_history_into_base_branch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let worktree = temp.path().join("worktree");
        init_repo(&repo_root);
        add_worktree(&repo_root, &worktree, "orbit/parallel-batch-jrun-1");

        std::fs::write(worktree.join("tracked.txt"), "task one\n").expect("write tracked.txt");
        run_git(&worktree, &["add", "tracked.txt"]);
        run_git(&worktree, &["commit", "-m", "task one"]);

        std::fs::write(worktree.join("second.txt"), "task two\n").expect("write second.txt");
        run_git(&worktree, &["add", "second.txt"]);
        run_git(&worktree, &["commit", "-m", "task two"]);

        let host = MockRuntimeHost {
            repo_root: repo_root.clone(),
            data_root: tempfile::tempdir().expect("data root"),
        };

        merge_batch_worktree_into_base(
            &host,
            &json!({
                "run_id": "jrun-1",
                "base": "main",
                "workspace_path": worktree.to_string_lossy().to_string(),
            }),
        )
        .expect("merge succeeds");

        assert_eq!(
            git_stdout(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"]),
            "main"
        );
        assert_eq!(
            git_stdout(&repo_root, &["rev-parse", "HEAD"]),
            git_stdout(&worktree, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            git_stdout(&repo_root, &["rev-list", "--count", "HEAD"]),
            "3"
        );
    }

    #[test]
    fn rejects_unresolved_merge_state_in_shared_worktree() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo_root = temp.path().join("repo");
        let worktree = temp.path().join("worktree");
        init_repo(&repo_root);
        add_worktree(&repo_root, &worktree, "orbit/parallel-batch-jrun-2");

        std::fs::write(worktree.join("tracked.txt"), "feature change\n").expect("write feature");
        run_git(&worktree, &["add", "tracked.txt"]);
        run_git(&worktree, &["commit", "-m", "feature change"]);

        std::fs::write(repo_root.join("tracked.txt"), "base change\n").expect("write base");
        run_git(&repo_root, &["add", "tracked.txt"]);
        run_git(&repo_root, &["commit", "-m", "base change"]);

        let worktree_str = worktree.to_string_lossy().into_owned();
        let merge = Command::new("git")
            .args(["-C", worktree_str.as_str(), "merge", "main"])
            .output()
            .expect("merge output");
        assert!(
            !merge.status.success(),
            "expected conflict, got: {}",
            String::from_utf8_lossy(&merge.stderr)
        );

        let host = MockRuntimeHost {
            repo_root: repo_root.clone(),
            data_root: tempfile::tempdir().expect("data root"),
        };

        let error = merge_batch_worktree_into_base(
            &host,
            &json!({
                "run_id": "jrun-2",
                "base": "main",
                "workspace_path": worktree.to_string_lossy().to_string(),
            }),
        )
        .expect_err("merge should fail");

        assert!(
            error.to_string().contains("unresolved merge conflicts"),
            "unexpected error: {error}"
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
