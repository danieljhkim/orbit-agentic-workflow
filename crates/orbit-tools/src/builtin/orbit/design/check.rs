use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitDesignCheckTool;

impl Tool for OrbitDesignCheckTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![
            ToolParam {
                name: "include_missing".to_string(),
                description:
                    "When true, missing referenced files are included in failure semantics for CLI parity."
                        .to_string(),
                param_type: "boolean".to_string(),
                required: false,
            },
            ToolParam {
                name: "workspace".to_string(),
                description:
                    "Optional workspace root containing docs/design/. Defaults to the server working directory."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
        ];
        parameters.extend(super::super::model_identity_params());
        ToolSchema {
            name: "orbit.design.check".to_string(),
            description:
                "Check docs/design/ decay and return structured stale-doc findings plus missing references."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::DesignCheck)
    }
}
