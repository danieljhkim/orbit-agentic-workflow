use orbit_types::OrbitError;
use serde_json::{Value, json};

use super::input::required_input_string;
use super::review::normalize_review_decision;
use crate::context::TaskHost;

pub(super) fn check_review_decision<H: TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;

    // Prefer direct pr_status piped from upstream review_pr output,
    // fall back to the persisted task field.
    let pr_status_raw = input
        .get("pr_status")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());

    let (pr_status, source) = if let Some(s) = pr_status_raw {
        (s.to_string(), "input")
    } else {
        let task = host.get_task(task_id)?;
        (
            task.pr_status.clone().unwrap_or_else(|| "none".to_string()),
            "task",
        )
    };

    let normalized = normalize_review_decision(&pr_status);
    if normalized == "APPROVED" {
        Ok(json!({ "review_decision": normalized }))
    } else {
        Err(OrbitError::Execution(format!(
            "task '{task_id}' is not approved (pr_status={pr_status}, source={source})"
        )))
    }
}
