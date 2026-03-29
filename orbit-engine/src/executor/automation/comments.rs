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
