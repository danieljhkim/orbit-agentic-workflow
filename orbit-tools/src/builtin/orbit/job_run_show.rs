use orbit_exec::ExecRequest;
use orbit_types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitJobRunShowTool;

pub(super) fn build_exec_request(
    ctx: &ToolContext,
    input: &Value,
) -> Result<ExecRequest, OrbitError> {
    let id = super::required_string(input, &["id", "run_id", "runId"], "id")?;
    Ok(super::orbit_exec_request(
        ctx,
        vec![
            "job-run".to_string(),
            "show".to_string(),
            id,
            "--json".to_string(),
        ],
    ))
}

impl Tool for OrbitJobRunShowTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.job_run.show".to_string(),
            description: "Fetch a single Orbit job run as JSON".to_string(),
            parameters: super::orbit_id_params("job run"),
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let req = build_exec_request(ctx, &input)?;
        super::run_orbit_json_command(req, "orbit job-run show")
    }
}
