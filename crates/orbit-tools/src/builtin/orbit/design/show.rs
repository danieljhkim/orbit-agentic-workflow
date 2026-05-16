use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitDesignShowTool;

impl Tool for OrbitDesignShowTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![ToolParam {
            name: "feature".to_string(),
            description: "Feature folder name under docs/design/.".to_string(),
            param_type: "string".to_string(),
            required: true,
        }, ToolParam {
            name: "workspace".to_string(),
            description:
                "Optional workspace root containing docs/design/. Defaults to the server working directory."
                    .to_string(),
            param_type: "string".to_string(),
            required: false,
        }];
        parameters.extend(super::super::model_identity_params());
        ToolSchema {
            name: "orbit.design.show".to_string(),
            description:
                "Show one design-doc feature with per-doc owner, Last updated date, decay status, and absolute paths."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::DesignShow)
    }
}
