use orbit_exec::{ExecRequest, NoSandbox, run_process};
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitTaskUpdateTool;

pub(super) fn build_exec_requests(
    ctx: &ToolContext,
    input: &Value,
) -> Result<(ExecRequest, ExecRequest), OrbitError> {
    let identity = super::resolve_identity(ctx, input)?;
    let id = super::required_string(input, &["id"], "id")?;
    let mut args = vec!["task".to_string(), "update".to_string(), id.clone()];
    let mut changed = false;

    if let Some(status) = super::optional_string(input, "status")? {
        args.push("--status".to_string());
        args.push(status);
        changed = true;
    }
    if let Some(plan) = input.get("plan") {
        let raw = plan
            .as_str()
            .ok_or_else(|| OrbitError::InvalidInput("`plan` must be a string".to_string()))?;
        args.push("--plan".to_string());
        args.push(raw.to_string());
        changed = true;
    }
    if let Some(summary) = super::optional_string(input, "execution_summary")? {
        args.push("--execution-summary".to_string());
        args.push(summary);
        changed = true;
    }
    if let Some(comment) = super::optional_string(input, "comment")? {
        args.push("--comment".to_string());
        args.push(comment);
        changed = true;
    }
    if let Some(pr_status) = super::optional_string(input, "pr_status")? {
        args.push("--pr-status".to_string());
        args.push(pr_status);
        changed = true;
    }
    if let Some(pr_number) = super::optional_string(input, "pr_number")? {
        args.push("--pr-number".to_string());
        args.push(pr_number);
        changed = true;
    }

    if !changed {
        return Err(OrbitError::InvalidInput(
            "orbit.task.update requires at least one of `status`, `plan`, `execution_summary`, `comment`, `pr_status`, or `pr_number`"
                .to_string(),
        ));
    }

    super::append_identity_flags(&mut args, &identity);

    let update = super::orbit_exec_request_with_identity(ctx, args, &identity);
    let show = super::orbit_exec_request_with_identity(
        ctx,
        vec![
            "task".to_string(),
            "show".to_string(),
            id,
            "--json".to_string(),
        ],
        &identity,
    );
    Ok((update, show))
}

impl Tool for OrbitTaskUpdateTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = super::orbit_id_params("task");
        parameters.extend([
            ToolParam {
                name: "plan".to_string(),
                description: "Replacement task plan text (empty string clears)".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "status".to_string(),
                description: "New task status".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "execution_summary".to_string(),
                description: "Replacement execution summary text".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "comment".to_string(),
                description: "Task comment to append".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "pr_status".to_string(),
                description: "PR review status (e.g. approve, request-changes)".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "pr_number".to_string(),
                description: "Pull request number (empty string clears)".to_string(),
                param_type: "string".to_string(),
                required: false,
            },
        ]);
        parameters.extend(super::identity_params());

        ToolSchema {
            name: "orbit.task.update".to_string(),
            description: "Update an Orbit task and return the fresh task JSON".to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let (update_req, show_req) = build_exec_requests(ctx, &input)?;

        let update_result = run_process(&update_req, &NoSandbox)?;
        if !update_result.success {
            let stderr = update_result.stderr.trim();
            let detail = if stderr.is_empty() {
                "command returned non-zero exit status"
            } else {
                stderr
            };
            return Err(OrbitError::Execution(format!(
                "orbit task update failed: {detail}"
            )));
        }

        super::run_orbit_json_command(show_req, "orbit task show")
    }
}
