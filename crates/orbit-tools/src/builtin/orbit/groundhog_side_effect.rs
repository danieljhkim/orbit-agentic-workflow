use orbit_common::groundhog::SideEffect;
use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{GroundhogBuiltinAction, Tool, ToolContext};

pub struct OrbitGroundhogSideEffectTool;

impl Tool for OrbitGroundhogSideEffectTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.groundhog.side_effect".to_string(),
            description: "Record a Groundhog side effect in the active runner context".to_string(),
            parameters: vec![
                ToolParam {
                    name: "kind".to_string(),
                    description: "Groundhog side-effect kind (for example `file_write`)"
                        .to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "target".to_string(),
                    description: "Affected path, ref, query id, or other side-effect target"
                        .to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "reversible".to_string(),
                    description: "Whether the side effect can be automatically reversed"
                        .to_string(),
                    param_type: "boolean".to_string(),
                    required: true,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::require_groundhog_fields(&input, "side_effect", &["kind", "target", "reversible"])?;
        let input: SideEffect = serde_json::from_value(input).map_err(|error| {
            OrbitError::InvalidInput(format!(
                "groundhog side_effect input validation failed: {error}"
            ))
        })?;
        super::execute_groundhog_action(
            ctx,
            GroundhogBuiltinAction::SideEffect,
            "side_effect",
            &input,
        )
    }
}
