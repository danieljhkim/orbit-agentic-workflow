use orbit_types::OrbitError;
use serde_json::{Value, json};

use crate::context::TaskHost;

use super::git::{git_output, git_success};
use super::input::{canonicalize_existing_dir, required_input_string};

pub(super) fn push_task_changes<H: TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let task = host.get_task(task_id)?;
    let workspace_path = canonicalize_existing_dir(
        task.workspace_path.as_deref().ok_or_else(|| {
            OrbitError::InvalidInput("push_task_changes requires task.workspace_path".to_string())
        })?,
        "workspace_path",
    )?;
    let expected_branch = format!("orbit/{task_id}");

    let actual_branch = git_output(&workspace_path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if actual_branch.trim() != expected_branch {
        return Err(OrbitError::Execution(format!(
            "workspace '{}' is on branch '{}' but '{}' was expected",
            workspace_path.display(),
            actual_branch.trim(),
            expected_branch
        )));
    }

    git_success(&workspace_path, &["push", "-u", "origin", &expected_branch])?;
    Ok(json!({}))
}
