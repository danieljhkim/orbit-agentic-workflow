use std::path::Path;

use orbit_types::OrbitError;
use serde_json::{Value, json};

use crate::context::{RuntimeHost, TaskHost};

use super::git::{git_output, git_success};
use super::input::{canonicalize_existing_dir, input_string_field, required_input_string};

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

pub(super) fn push_batch_changes<H: RuntimeHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let workspace_path = match input_string_field(input, "workspace_path") {
        Some(ws) => canonicalize_existing_dir(&ws, "workspace_path")?,
        None => {
            let repo_root_str = host.repo_root()?;
            let repo_root = Path::new(&repo_root_str);
            super::parallel::resolve_shared_worktree_path(repo_root)?
        }
    };

    let branch = git_output(&workspace_path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let branch = branch.trim().to_string();

    if branch == "HEAD" {
        return Err(OrbitError::Execution(
            "push_batch_changes: workspace is in detached HEAD state".to_string(),
        ));
    }

    git_success(&workspace_path, &["push", "-u", "origin", &branch])?;
    Ok(json!({}))
}
