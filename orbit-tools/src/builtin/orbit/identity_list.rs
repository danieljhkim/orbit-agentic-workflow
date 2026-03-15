use orbit_exec::ExecRequest;
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitIdentityListTool;

pub(super) fn build_exec_request(
    ctx: &ToolContext,
    input: &Value,
) -> Result<ExecRequest, OrbitError> {
    let mut args = vec![
        "identity".to_string(),
        "list".to_string(),
        "--json".to_string(),
    ];

    if let Some(role) = super::optional_string(input, "role")? {
        args.push("--role".to_string());
        args.push(role);
    }

    Ok(super::orbit_exec_request(ctx, args))
}

impl Tool for OrbitIdentityListTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.identity.list".to_string(),
            description: "List Orbit identities as JSON".to_string(),
            parameters: vec![ToolParam {
                name: "role".to_string(),
                description: "Optional identity role filter".to_string(),
                param_type: "string".to_string(),
                required: false,
            }],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let req = build_exec_request(ctx, &input)?;
        let output = super::run_orbit_json_command(ctx, req.args, "orbit identity list")?;
        if !output.is_array() {
            return Err(OrbitError::Execution(
                "failed to parse orbit identity list output: expected JSON array".to_string(),
            ));
        }
        Ok(output)
    }
}
