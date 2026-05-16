use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitFrictionListTool;

impl Tool for OrbitFrictionListTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.friction.list".to_string(),
            description: "List Orbit friction records from .orbit/frictions/".to_string(),
            parameters: vec![
                ToolParam {
                    name: "model".to_string(),
                    description: "Optional model filter".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "tag".to_string(),
                    description: "Optional tag filter".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "from".to_string(),
                    description: "Optional RFC3339 lower bound for created_at".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "to".to_string(),
                    description: "Optional RFC3339 upper bound for created_at".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::FrictionList)
    }
}
