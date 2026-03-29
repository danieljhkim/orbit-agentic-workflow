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
    let task = host.get_task(task_id)?;

    if task.pr_number.is_none() {
        return Ok(json!({ "review_decision": "SKIPPED" }));
    }

    let pr_status = task.pr_status.clone().unwrap_or_else(|| "none".to_string());

    let normalized = normalize_review_decision(&pr_status);
    if normalized == "APPROVED" {
        Ok(json!({ "review_decision": normalized }))
    } else {
        Err(OrbitError::Execution(format!(
            "task '{task_id}' is not approved (pr_status={pr_status})"
        )))
    }
}
