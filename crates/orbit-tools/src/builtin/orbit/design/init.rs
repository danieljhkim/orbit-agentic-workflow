use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitDesignInitTool;

impl Tool for OrbitDesignInitTool {
    fn schema(&self) -> ToolSchema {
        let mut parameters = vec![
            ToolParam {
                name: "feature".to_string(),
                description:
                    "Feature folder name to create under docs/design/ (lowercase, hyphenated)."
                        .to_string(),
                param_type: "string".to_string(),
                required: true,
            },
            ToolParam {
                name: "owner".to_string(),
                description:
                    "Optional owner to write into the numbered-doc frontmatter. Defaults to the caller identity."
                        .to_string(),
                param_type: "string".to_string(),
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
            name: "orbit.design.init".to_string(),
            description:
                "Scaffold docs/design/<feature>/ with the four numbered design docs plus empty specs/ and references/ folders."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::DesignInit)
    }
}
