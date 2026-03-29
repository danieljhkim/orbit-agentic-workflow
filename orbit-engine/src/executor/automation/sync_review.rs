use orbit_exec::{EnvironmentMode, ExecRequest, NoSandbox, StdinMode, run_process};
use orbit_types::{OrbitError, ReviewThread};
use serde_json::{Value, json};

use super::input::required_input_string;
use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};

const TIMEOUT_MS: u64 = 15_000;

pub(super) fn sync_review_to_github<H: RuntimeHost + TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let task = host.get_task(task_id)?;

    if task.pr_number.is_none() {
        return Ok(json!({ "synced_count": 0 }));
    }

    if task.review_threads.is_empty() {
        return Ok(json!({ "synced_count": 0 }));
    }

    let pr_number = task.pr_number.as_deref().unwrap();
    let repo_root = task
        .repo_root
        .as_deref()
        .or(task.workspace_path.as_deref())
        .ok_or_else(|| {
            OrbitError::InvalidInput(
                "sync_review_to_github requires task.repo_root or task.workspace_path".to_string(),
            )
        })?;

    let owner_repo = get_owner_repo(repo_root)?;
    let head_sha = get_pr_head_sha(repo_root, pr_number)?;

    let mut threads = task.review_threads.clone();
    let mut synced_count: u64 = 0;

    for thread in threads.iter_mut() {
        let thread_synced =
            sync_thread(repo_root, &owner_repo, pr_number, &head_sha, thread)?;
        synced_count += thread_synced;
    }

    if synced_count > 0 {
        host.apply_task_automation_update(
            task_id,
            TaskAutomationUpdate {
                review_threads: Some(threads),
                ..TaskAutomationUpdate::default()
            },
        )?;
    }

    Ok(json!({ "synced_count": synced_count }))
}

fn sync_thread(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
    head_sha: &str,
    thread: &mut ReviewThread,
) -> Result<u64, OrbitError> {
    let mut synced: u64 = 0;

    if thread.github_thread_id.is_none() && !thread.messages.is_empty() {
        let first_msg = &thread.messages[0];

        let github_id = if thread.path.is_some() && thread.line.is_some() {
            // Inline review comment
            create_inline_review_comment(
                repo_root,
                owner_repo,
                pr_number,
                head_sha,
                thread.path.as_deref().unwrap(),
                thread.line.unwrap(),
                &first_msg.body,
            )?
        } else {
            // General PR comment
            create_general_comment(repo_root, pr_number, &first_msg.body)?
        };

        thread.github_thread_id = Some(github_id);
        thread.messages[0].github_comment_id = Some(github_id);
        synced += 1;
    }

    // Sync reply messages on already-synced threads
    if let Some(parent_id) = thread.github_thread_id {
        for msg in thread.messages.iter_mut().skip(1) {
            if msg.github_comment_id.is_some() {
                continue;
            }
            let reply_id =
                create_reply_comment(repo_root, owner_repo, pr_number, parent_id, &msg.body)?;
            msg.github_comment_id = Some(reply_id);
            synced += 1;
        }
    }

    Ok(synced)
}

fn get_owner_repo(repo_root: &str) -> Result<String, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "repo".to_string(),
                "view".to_string(),
                "--json".to_string(),
                "nameWithOwner".to_string(),
                "-q".to_string(),
                ".nameWithOwner".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to get repo owner/name: {}",
            result.stderr.trim()
        )));
    }

    Ok(result.stdout.trim().to_string())
}

fn get_pr_head_sha(repo_root: &str, pr_number: &str) -> Result<String, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "pr".to_string(),
                "view".to_string(),
                pr_number.to_string(),
                "--json".to_string(),
                "headRefOid".to_string(),
                "-q".to_string(),
                ".headRefOid".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to get PR head SHA: {}",
            result.stderr.trim()
        )));
    }

    Ok(result.stdout.trim().to_string())
}

fn create_inline_review_comment(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
    commit_id: &str,
    path: &str,
    line: u64,
    body: &str,
) -> Result<u64, OrbitError> {
    let payload = json!({
        "body": body,
        "commit_id": commit_id,
        "path": path,
        "line": line,
    });

    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "api".to_string(),
                format!("repos/{owner_repo}/pulls/{pr_number}/comments"),
                "--method".to_string(),
                "POST".to_string(),
                "--input".to_string(),
                "-".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Bytes(payload.to_string().into_bytes()),
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to create inline review comment: {}",
            result.stderr.trim()
        )));
    }

    parse_comment_id(&result.stdout)
}

fn create_general_comment(
    repo_root: &str,
    pr_number: &str,
    body: &str,
) -> Result<u64, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "pr".to_string(),
                "comment".to_string(),
                pr_number.to_string(),
                "--body".to_string(),
                body.to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to create PR comment: {}",
            result.stderr.trim()
        )));
    }

    // gh pr comment outputs a URL like https://github.com/owner/repo/pull/1#issuecomment-123
    // but doesn't return structured JSON by default. Use the API instead.
    // Actually, let's use the API for general comments too for consistency.
    // Fall back: parse the URL for the comment ID, or return 0 if unparseable.
    // For general comments via `gh pr comment`, the output is a URL.
    // Extract comment ID from the URL fragment.
    let output = result.stdout.trim();
    if let Some(id_str) = output.rsplit("issuecomment-").nth(0) {
        if let Ok(id) = id_str.trim().parse::<u64>() {
            return Ok(id);
        }
    }

    // If we can't parse the ID from the URL, return an error rather than silently losing it
    Err(OrbitError::Execution(format!(
        "could not parse comment ID from gh pr comment output: {output}"
    )))
}

fn create_reply_comment(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
    parent_comment_id: u64,
    body: &str,
) -> Result<u64, OrbitError> {
    let payload = json!({ "body": body });

    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "api".to_string(),
                format!(
                    "repos/{owner_repo}/pulls/{pr_number}/comments/{parent_comment_id}/replies"
                ),
                "--method".to_string(),
                "POST".to_string(),
                "--input".to_string(),
                "-".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Bytes(payload.to_string().into_bytes()),
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to create reply comment: {}",
            result.stderr.trim()
        )));
    }

    parse_comment_id(&result.stdout)
}

fn parse_comment_id(json_output: &str) -> Result<u64, OrbitError> {
    let value: Value = serde_json::from_str(json_output.trim()).map_err(|e| {
        OrbitError::Execution(format!("failed to parse GitHub API response: {e}"))
    })?;

    value
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| OrbitError::Execution("GitHub API response missing 'id' field".to_string()))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;

    use chrono::Utc;
    use orbit_tools::ToolContext;
    use orbit_types::{
        Activity, ActorIdentity, JobTargetType, OrbitError, OrbitEvent, ReviewMessage,
        ReviewThread, ReviewThreadStatus, Role, Task, TaskPriority, TaskStatus, TaskType,
    };
    use serde_json::{Value, json};
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};

    struct FakeHost {
        task: RefCell<Option<Task>>,
        automation_updates: RefCell<Vec<TaskAutomationUpdate>>,
    }

    impl FakeHost {
        fn new(task: Task) -> Self {
            Self {
                task: RefCell::new(Some(task)),
                automation_updates: RefCell::new(Vec::new()),
            }
        }
    }

    impl TaskHost for FakeHost {
        fn get_task(&self, task_id: &str) -> Result<Task, OrbitError> {
            let task = self
                .task
                .borrow()
                .clone()
                .ok_or_else(|| OrbitError::TaskNotFound(task_id.to_string()))?;
            if task.id != task_id {
                return Err(OrbitError::TaskNotFound(task_id.to_string()));
            }
            Ok(task)
        }

        fn start_task(
            &self,
            _task_id: &str,
            _note: Option<String>,
            _comment: Option<String>,
        ) -> Result<Task, OrbitError> {
            unimplemented!()
        }

        fn update_task_from_activity(
            &self,
            _task_id: &str,
            _status: TaskStatus,
            _execution_summary: Option<String>,
            _comment: Option<String>,
            _note: Option<String>,
        ) -> Result<Task, OrbitError> {
            unimplemented!()
        }

        fn apply_task_automation_update(
            &self,
            _task_id: &str,
            update: TaskAutomationUpdate,
        ) -> Result<(), OrbitError> {
            self.automation_updates.borrow_mut().push(update);
            Ok(())
        }
    }

    impl RuntimeHost for FakeHost {
        fn record_event(&self, _event: OrbitEvent) -> Result<(), OrbitError> {
            Ok(())
        }

        fn repo_root(&self) -> Result<String, OrbitError> {
            Err(OrbitError::Execution("not used".to_string()))
        }

        fn data_root(&self) -> &Path {
            Path::new(".")
        }

        fn validate_activity_target_exists(
            &self,
            _target_type: JobTargetType,
            _target_id: &str,
        ) -> Result<Activity, OrbitError> {
            unimplemented!()
        }

        fn get_job(&self, _job_id: &str) -> Result<Option<orbit_types::Job>, OrbitError> {
            Ok(None)
        }

        fn run_tool_with_context_and_role(
            &self,
            _name: &str,
            _input: Value,
            _role: Role,
            _tool_context: ToolContext,
        ) -> Result<Value, OrbitError> {
            Ok(json!({}))
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
            Ok(())
        }

        fn scoring_enabled(&self) -> bool {
            false
        }

        fn scoreboard_dir(&self) -> &Path {
            Path::new(".")
        }
    }

    fn test_task(repo_root: &Path) -> Task {
        Task {
            id: "T20260329-010000".to_string(),
            parent_id: None,
            title: "test sync review".to_string(),
            description: "desc".to_string(),
            plan: "plan".to_string(),
            execution_summary: String::new(),
            context_files: vec![],
            workspace_path: Some(repo_root.to_string_lossy().to_string()),
            repo_root: Some(repo_root.to_string_lossy().to_string()),
            assigned_to: None,
            created_by: Some("test".to_string()),
            actor_identity: ActorIdentity::agent("claude", "opus-4.6"),
            status: TaskStatus::Review,
            priority: TaskPriority::High,
            task_type: TaskType::Issue,
            pr_number: Some("42".to_string()),
            pr_status: None,
            proposed_by: None,
            source_task_id: None,
            complexity: None,
            comments: vec![],
            history: vec![],
            review_threads: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn sync_returns_zero_when_no_review_threads() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let host = FakeHost::new(test_task(repo_dir.path()));

        let result = sync_review_to_github(&host, &json!({"task_id": "T20260329-010000"}))
            .expect("should succeed");

        assert_eq!(result["synced_count"], json!(0));
        assert!(host.automation_updates.borrow().is_empty());
    }

    #[test]
    fn sync_returns_zero_when_no_pr_number() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.pr_number = None;
        task.review_threads = vec![ReviewThread {
            thread_id: "t1".to_string(),
            path: None,
            line: None,
            status: ReviewThreadStatus::Open,
            messages: vec![ReviewMessage {
                message_id: "m1".to_string(),
                at: Utc::now(),
                by: "reviewer".to_string(),
                body: "fix this".to_string(),
                github_comment_id: None,
            }],
            github_thread_id: None,
        }];
        let host = FakeHost::new(task);

        let result = sync_review_to_github(&host, &json!({"task_id": "T20260329-010000"}))
            .expect("should succeed");

        assert_eq!(result["synced_count"], json!(0));
        assert!(host.automation_updates.borrow().is_empty());
    }

    #[test]
    fn sync_returns_zero_when_all_threads_already_synced() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![ReviewThread {
            thread_id: "t1".to_string(),
            path: None,
            line: None,
            status: ReviewThreadStatus::Open,
            messages: vec![ReviewMessage {
                message_id: "m1".to_string(),
                at: Utc::now(),
                by: "reviewer".to_string(),
                body: "fix this".to_string(),
                github_comment_id: Some(100),
            }],
            github_thread_id: Some(100),
        }];

        // Install a fake gh that responds to repo view and pr view
        let bin_dir = tempfile::tempdir().expect("temp gh dir");
        install_fake_gh(bin_dir.path());
        let lock = path_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let original_path = std::env::var("PATH").ok();
        unsafe { std::env::set_var("PATH", prepend_path(bin_dir.path())) };

        let host = FakeHost::new(task);
        let result = sync_review_to_github(&host, &json!({"task_id": "T20260329-010000"}))
            .expect("should succeed");

        // Restore PATH
        match original_path {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
        drop(lock);

        assert_eq!(result["synced_count"], json!(0));
        assert!(host.automation_updates.borrow().is_empty());
    }

    fn path_lock() -> &'static Mutex<()> {
        crate::executor::automation::test_utils::path_lock()
    }

    fn prepend_path(dir: &Path) -> String {
        let mut entries = vec![dir.to_string_lossy().to_string()];
        if let Some(existing) = std::env::var_os("PATH") {
            entries.push(existing.to_string_lossy().to_string());
        }
        entries.join(":")
    }

    fn install_fake_gh(bin_dir: &Path) {
        let script = concat!(
            "#!/bin/sh\n",
            "# Fake gh for sync_review tests\n",
            "if [ \"$1\" = \"repo\" ] && [ \"$2\" = \"view\" ]; then\n",
            "  printf 'owner/repo'\n",
            "  exit 0\n",
            "fi\n",
            "if [ \"$1\" = \"pr\" ] && [ \"$2\" = \"view\" ]; then\n",
            "  printf 'abc123def'\n",
            "  exit 0\n",
            "fi\n",
            "if [ \"$1\" = \"api\" ]; then\n",
            "  printf '{\"id\": 999}'\n",
            "  exit 0\n",
            "fi\n",
            "if [ \"$1\" = \"pr\" ] && [ \"$2\" = \"comment\" ]; then\n",
            "  printf 'https://github.com/owner/repo/pull/42#issuecomment-999'\n",
            "  exit 0\n",
            "fi\n",
            "printf '%s\\n' \"unexpected gh args: $*\" >&2\n",
            "exit 1\n",
        );
        let gh_path = bin_dir.join("gh");
        fs::write(&gh_path, script).expect("write fake gh");
        #[cfg(unix)]
        fs::set_permissions(&gh_path, fs::Permissions::from_mode(0o755)).expect("chmod gh");
    }

    struct PathGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        original_path: Option<String>,
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            match self.original_path.take() {
                Some(path) => unsafe { std::env::set_var("PATH", path) },
                None => unsafe { std::env::remove_var("PATH") },
            }
        }
    }

    fn use_fake_gh() -> (TempDir, PathGuard) {
        let bin_dir = tempfile::tempdir().expect("temp gh dir");
        install_fake_gh(bin_dir.path());
        let lock = path_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let original_path = std::env::var("PATH").ok();
        unsafe { std::env::set_var("PATH", prepend_path(bin_dir.path())) };
        (
            bin_dir,
            PathGuard {
                _lock: lock,
                original_path,
            },
        )
    }

    #[test]
    fn sync_posts_unsynced_general_comment() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![ReviewThread {
            thread_id: "t1".to_string(),
            path: None,
            line: None,
            status: ReviewThreadStatus::Open,
            messages: vec![ReviewMessage {
                message_id: "m1".to_string(),
                at: Utc::now(),
                by: "reviewer".to_string(),
                body: "general comment".to_string(),
                github_comment_id: None,
            }],
            github_thread_id: None,
        }];

        let (_gh_dir, _path_guard) = use_fake_gh();
        let host = FakeHost::new(task);

        let result = sync_review_to_github(&host, &json!({"task_id": "T20260329-010000"}))
            .expect("should succeed");

        assert_eq!(result["synced_count"], json!(1));
        let updates = host.automation_updates.borrow();
        assert_eq!(updates.len(), 1);
        let threads = updates[0].review_threads.as_ref().expect("should have threads");
        assert_eq!(threads.len(), 1);
        assert!(threads[0].github_thread_id.is_some());
        assert!(threads[0].messages[0].github_comment_id.is_some());
    }

    #[test]
    fn sync_posts_unsynced_inline_comment() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![ReviewThread {
            thread_id: "t1".to_string(),
            path: Some("src/main.rs".to_string()),
            line: Some(42),
            status: ReviewThreadStatus::Open,
            messages: vec![ReviewMessage {
                message_id: "m1".to_string(),
                at: Utc::now(),
                by: "reviewer".to_string(),
                body: "inline comment".to_string(),
                github_comment_id: None,
            }],
            github_thread_id: None,
        }];

        let (_gh_dir, _path_guard) = use_fake_gh();
        let host = FakeHost::new(task);

        let result = sync_review_to_github(&host, &json!({"task_id": "T20260329-010000"}))
            .expect("should succeed");

        assert_eq!(result["synced_count"], json!(1));
        let updates = host.automation_updates.borrow();
        assert_eq!(updates.len(), 1);
        let threads = updates[0].review_threads.as_ref().expect("should have threads");
        assert!(threads[0].github_thread_id.is_some());
    }

    #[test]
    fn sync_posts_reply_on_synced_thread() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![ReviewThread {
            thread_id: "t1".to_string(),
            path: Some("src/main.rs".to_string()),
            line: Some(10),
            status: ReviewThreadStatus::Open,
            messages: vec![
                ReviewMessage {
                    message_id: "m1".to_string(),
                    at: Utc::now(),
                    by: "reviewer".to_string(),
                    body: "original comment".to_string(),
                    github_comment_id: Some(100),
                },
                ReviewMessage {
                    message_id: "m2".to_string(),
                    at: Utc::now(),
                    by: "agent".to_string(),
                    body: "reply".to_string(),
                    github_comment_id: None,
                },
            ],
            github_thread_id: Some(100),
        }];

        let (_gh_dir, _path_guard) = use_fake_gh();
        let host = FakeHost::new(task);

        let result = sync_review_to_github(&host, &json!({"task_id": "T20260329-010000"}))
            .expect("should succeed");

        assert_eq!(result["synced_count"], json!(1));
        let updates = host.automation_updates.borrow();
        assert_eq!(updates.len(), 1);
        let threads = updates[0].review_threads.as_ref().expect("should have threads");
        assert!(threads[0].messages[1].github_comment_id.is_some());
    }
}
