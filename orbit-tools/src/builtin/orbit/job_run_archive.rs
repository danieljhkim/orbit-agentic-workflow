use orbit_exec::{ExecRequest, NoSandbox, run_process};
use orbit_types::{OrbitError, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct OrbitJobRunArchiveTool;

pub(super) fn build_exec_request(
    ctx: &ToolContext,
    input: &Value,
) -> Result<ExecRequest, OrbitError> {
    let id = super::required_string(input, &["id", "run_id", "runId"], "id")?;
    Ok(super::orbit_exec_request(
        ctx,
        vec!["job-run".to_string(), "archive".to_string(), id],
    ))
}

impl Tool for OrbitJobRunArchiveTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.job_run.archive".to_string(),
            description: "Archive an Orbit job run".to_string(),
            parameters: super::orbit_id_params("job run"),
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let req = build_exec_request(ctx, &input)?;
        let archived_id = req
            .args
            .get(2)
            .cloned()
            .ok_or_else(|| OrbitError::Execution("missing job run id".to_string()))?;
        let result = run_process(&req, &NoSandbox)?;
        if !result.success {
            let stderr = result.stderr.trim();
            let detail = if stderr.is_empty() {
                "command returned non-zero exit status"
            } else {
                stderr
            };
            return Err(OrbitError::Execution(format!(
                "orbit job-run archive failed: {detail}"
            )));
        }

        Ok(json!({
            "archived": true,
            "id": archived_id,
        }))
    }
}
