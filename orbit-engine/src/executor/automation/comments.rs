use orbit_store::pr_scoreboard;
use orbit_types::{OrbitError, ReviewThreadStatus};
use serde_json::{Value, json};

use super::input::required_input_string;
use crate::context::{RuntimeHost, TaskHost};

pub(super) fn load_pr_comments<H: RuntimeHost + TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    // If upstream review_pr approved, exit the loop immediately.
    if let Some(status) = input.get("pr_status").and_then(Value::as_str) {
        let normalized = super::review::normalize_review_decision(status);
        if normalized == "APPROVED" {
            return Ok(json!({
                "loop_exit": true,
                "comments": [],
                "comment_summary": "PR approved — no further fixes needed.",
            }));
        }
    }

    let task_id = required_input_string(input, "task_id")?;
    let task = host.get_task(task_id)?;

    // Read open review threads from the task (written by review_pr via orbit.task.review_thread.add).
    let open_threads: Vec<_> = task
        .review_threads
        .iter()
        .filter(|t| t.status == ReviewThreadStatus::Open)
        .collect();

    if open_threads.is_empty() {
        return Ok(json!({
            "loop_exit": true,
            "comments": [],
            "comment_summary": "No unresolved review threads.",
        }));
    }

    // Record a PR revision in the scoreboard (tracks how many fix rounds).
    if host.scoring_enabled()
        && let (Some(agent), Some(model)) = (
            task.actor_identity.agent_name(),
            task.actor_identity.agent_model(),
        )
    {
        let _ = pr_scoreboard::record_pr_revision(host.scoreboard_dir(), agent, model);
    }

    // Convert review threads to a comment-like JSON array for implement_fix consumption.
    let comments: Vec<Value> = open_threads
        .iter()
        .map(|t| {
            let body = t
                .messages
                .iter()
                .map(|m| format!("[{}] {}", m.by, m.body))
                .collect::<Vec<_>>()
                .join("\n\n");
            json!({
                "thread_id": t.thread_id,
                "path": t.path,
                "line": t.line,
                "body": body,
                "status": t.status.to_string(),
                "message_count": t.messages.len(),
            })
        })
        .collect();

    let summary = build_comment_summary(&comments);
    Ok(json!({
        "loop_exit": false,
        "comments": comments,
        "comment_summary": summary,
    }))
}

fn build_comment_summary(comments: &[Value]) -> String {
    let mut summary = format!("{} unresolved review thread(s):\n", comments.len());
    for (i, comment) in comments.iter().enumerate() {
        let path = comment
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("general");
        let line = comment
            .get("line")
            .and_then(Value::as_u64)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let body = comment
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>();
        let thread_id = comment
            .get("thread_id")
            .and_then(Value::as_str)
            .unwrap_or("?");
        summary.push_str(&format!(
            "\n{}. {} ({}:{}) — {}\n",
            i + 1,
            thread_id,
            path,
            line,
            body
        ));
    }
    summary
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use orbit_tools::ToolContext;
    use orbit_types::{
        Activity, ActorIdentity, JobTargetType, OrbitError, OrbitEvent, ReviewMessage,
        ReviewThread, ReviewThreadStatus, Role, Task, TaskPriority, TaskStatus, TaskType,
    };
    use serde_json::{Value, json};

    use super::*;
    use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};

    struct FakeHost {
        task: std::cell::RefCell<Option<Task>>,
        scoring_enabled: bool,
        scoreboard_dir: std::path::PathBuf,
    }

    impl FakeHost {
        fn new(task: Task) -> Self {
            let scoreboard_dir = task
                .repo_root
                .as_deref()
                .or(task.workspace_path.as_deref())
                .map(|p| std::path::Path::new(p).join(".orbit").join("scoreboard"))
                .unwrap_or_default();
            Self {
                task: std::cell::RefCell::new(Some(task)),
                scoring_enabled: false,
                scoreboard_dir,
            }
        }

        fn with_scoring(mut self) -> Self {
            self.scoring_enabled = true;
            self
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
            _update: TaskAutomationUpdate,
        ) -> Result<(), OrbitError> {
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

        fn data_root(&self) -> &std::path::Path {
            std::path::Path::new(".")
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
            self.scoring_enabled
        }

        fn scoreboard_dir(&self) -> &std::path::Path {
            &self.scoreboard_dir
        }
    }

    fn test_task(repo_root: &std::path::Path) -> Task {
        Task {
            id: "T20260320-021158".to_string(),
            parent_id: None,
            title: "test task".to_string(),
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

    fn make_thread(id: &str, path: Option<&str>, line: Option<u64>, status: ReviewThreadStatus, body: &str) -> ReviewThread {
        ReviewThread {
            thread_id: id.to_string(),
            path: path.map(String::from),
            line,
            status,
            messages: vec![ReviewMessage {
                message_id: format!("rm-{id}"),
                at: Utc::now(),
                by: "reviewer".to_string(),
                body: body.to_string(),
                github_comment_id: None,
            }],
            github_thread_id: None,
        }
    }

    #[test]
    fn load_pr_comments_returns_open_threads() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![
            make_thread("rt-1", Some("src/main.rs"), Some(10), ReviewThreadStatus::Open, "fix this"),
            make_thread("rt-2", Some("src/lib.rs"), Some(20), ReviewThreadStatus::Open, "and this"),
        ];
        let host = FakeHost::new(task);

        let result = load_pr_comments(&host, &json!({"task_id": "T20260320-021158"}))
            .expect("should succeed");

        assert_eq!(result["loop_exit"], json!(false));
        assert_eq!(result["comments"].as_array().unwrap().len(), 2);
        assert!(result["comment_summary"].as_str().unwrap().contains("2 unresolved"));
    }

    #[test]
    fn load_pr_comments_exits_when_all_resolved() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![
            make_thread("rt-1", Some("src/main.rs"), Some(10), ReviewThreadStatus::Resolved, "fixed"),
        ];
        let host = FakeHost::new(task);

        let result = load_pr_comments(&host, &json!({"task_id": "T20260320-021158"}))
            .expect("should succeed");

        assert_eq!(result["loop_exit"], json!(true));
        assert_eq!(result["comments"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn load_pr_comments_exits_on_approve_status() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let host = FakeHost::new(test_task(repo_dir.path()));

        let result = load_pr_comments(&host, &json!({"task_id": "T20260320-021158", "pr_status": "approve"}))
            .expect("should succeed");

        assert_eq!(result["loop_exit"], json!(true));
    }

    #[test]
    fn load_pr_comments_records_scoreboard_for_open_threads() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![
            make_thread("rt-1", Some("src/main.rs"), Some(10), ReviewThreadStatus::Open, "fix"),
        ];
        let host = FakeHost::new(task).with_scoring();

        let result = load_pr_comments(&host, &json!({"task_id": "T20260320-021158"}))
            .expect("should succeed");

        assert_eq!(result["loop_exit"], json!(false));

        // Check scoreboard was written
        let sb_path = repo_dir.path().join(".orbit/scoreboard/pr.json");
        assert!(sb_path.exists(), "scoreboard should exist");
    }

    #[test]
    fn load_pr_comments_no_scoreboard_when_agent_missing() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.actor_identity = ActorIdentity::System;
        task.review_threads = vec![
            make_thread("rt-1", None, None, ReviewThreadStatus::Open, "general comment"),
        ];
        let host = FakeHost::new(task);

        let result = load_pr_comments(&host, &json!({"task_id": "T20260320-021158"}))
            .expect("should succeed");

        assert_eq!(result["loop_exit"], json!(false));

        let sb_path = repo_dir.path().join(".orbit/scoreboard/pr.json");
        assert!(!sb_path.exists(), "scoreboard should not exist");
    }

    #[test]
    fn load_pr_comments_filters_only_open_threads() {
        let repo_dir = tempfile::tempdir().expect("temp dir");
        let mut task = test_task(repo_dir.path());
        task.review_threads = vec![
            make_thread("rt-1", Some("a.rs"), Some(1), ReviewThreadStatus::Open, "open issue"),
            make_thread("rt-2", Some("b.rs"), Some(2), ReviewThreadStatus::Resolved, "done"),
            make_thread("rt-3", None, None, ReviewThreadStatus::Open, "general"),
        ];
        let host = FakeHost::new(task);

        let result = load_pr_comments(&host, &json!({"task_id": "T20260320-021158"}))
            .expect("should succeed");

        let comments = result["comments"].as_array().unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0]["thread_id"], "rt-1");
        assert_eq!(comments[1]["thread_id"], "rt-3");
    }
}
