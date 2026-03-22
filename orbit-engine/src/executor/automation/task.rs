use orbit_types::{OrbitError, TaskStatus};
use serde_json::{Value, json};

use crate::context::TaskHost;

use super::input::{input_string_field, required_input_string};

pub(super) fn start_task<H: TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let note = input_string_field(input, "note")
        .or_else(|| Some("automation: start_task → in-progress".to_string()));
    let task = host.start_task(
        task_id,
        note,
        input_string_field(input, "comment"),
    )?;
    Ok(json!({
        "task_id": task.id.to_string(),
        "status": task.status,
    }))
}

pub(super) fn update_task<H: TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let status = required_input_string(input, "status")?
        .parse::<TaskStatus>()
        .map_err(|error| OrbitError::InvalidInput(format!("invalid input.status: {error}")))?;
    let note = input_string_field(input, "note")
        .or_else(|| Some(format!("automation: update_task → {status}")));
    let task = host.update_task_from_activity(
        task_id,
        status,
        input_string_field(input, "execution_summary"),
        input_string_field(input, "comment"),
        note,
    )?;
    Ok(json!({
        "task_id": task.id.to_string(),
    }))
}
