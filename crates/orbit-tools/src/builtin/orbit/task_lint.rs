use orbit_exec::ExecRequest;
use orbit_types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitTaskLintTool;

pub(super) fn build_exec_request(
    ctx: &ToolContext,
    input: &Value,
) -> Result<ExecRequest, OrbitError> {
    let identity = super::resolve_identity(ctx, input)?;
    let id = super::required_string(input, &["id"], "id")?;
    let mut args = vec![
        "task".to_string(),
        "lint".to_string(),
        id,
        "--json".to_string(),
    ];
    super::append_identity_flags(&mut args, &identity);
    Ok(super::orbit_exec_request_with_identity(
        ctx, args, &identity,
    ))
}

impl Tool for OrbitTaskLintTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = super::orbit_id_params("task");
        parameters.extend(super::identity_params());
        ToolSchema {
            name: "orbit.task.lint".to_string(),
            description: "Lint an Orbit task for stale paths, vague acceptance criteria, and stale repository names.".to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let req = build_exec_request(ctx, &input)?;
        super::run_orbit_json_command(req, "orbit task lint")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use crate::ToolContext;

    use super::build_exec_request;

    fn test_context() -> ToolContext {
        ToolContext {
            cwd: Some("/tmp/orbit".to_string()),
            orbit_root: Some(PathBuf::from("/tmp/orbit-root")),
            agent_name: Some("codex".to_string()),
            model_name: Some("gpt-5.4".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn build_exec_request_uses_task_lint_subcommand() {
        let request = build_exec_request(&test_context(), &json!({ "id": "T20260408-0503" }))
            .expect("request should build");

        assert_eq!(
            request.args,
            vec![
                "--root",
                "/tmp/orbit-root",
                "task",
                "lint",
                "T20260408-0503",
                "--json",
                "--agent",
                "codex",
                "--model",
                "gpt-5.4",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        );
    }
}
