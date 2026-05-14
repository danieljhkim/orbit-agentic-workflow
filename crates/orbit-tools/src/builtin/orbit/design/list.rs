use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitDesignListTool;

impl Tool for OrbitDesignListTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![ToolParam {
            name: "workspace".to_string(),
            description:
                "Optional workspace root containing docs/design/. Defaults to the server working directory."
                    .to_string(),
            param_type: "string".to_string(),
            required: false,
        }];
        parameters.extend(super::super::model_identity_params());
        ToolSchema {
            name: "orbit.design.list".to_string(),
            description:
                "List docs/design feature folders with owner, Last updated, decay status, and paths for numbered docs."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::DesignList)
    }
}
