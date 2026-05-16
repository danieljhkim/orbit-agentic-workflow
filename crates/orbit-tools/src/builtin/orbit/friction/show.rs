use orbit_common::types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitFrictionShowTool;

impl Tool for OrbitFrictionShowTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.friction.show".to_string(),
            description: "Fetch a single Orbit friction record by id".to_string(),
            parameters: super::super::orbit_id_params("friction"),
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::FrictionShow)
    }
}
