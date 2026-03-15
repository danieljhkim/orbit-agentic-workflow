use orbit_exec::ExecRequest;
use orbit_types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitIdentityShowTool;

pub(super) fn build_exec_request(
    ctx: &ToolContext,
    input: &Value,
) -> Result<ExecRequest, OrbitError> {
    let id = super::required_string(input, &["id", "identity_id", "identityId"], "id")?;
    Ok(super::orbit_exec_request(
        ctx,
        vec![
            "identity".to_string(),
            "show".to_string(),
            id,
            "--json".to_string(),
        ],
    ))
}

impl Tool for OrbitIdentityShowTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.identity.show".to_string(),
            description: "Fetch a single Orbit identity as JSON".to_string(),
            parameters: super::orbit_id_params("identity"),
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let req = build_exec_request(ctx, &input)?;
        super::run_orbit_json_command(ctx, req.args, "orbit identity show")
    }
}
