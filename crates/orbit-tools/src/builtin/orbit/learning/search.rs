use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningSearchTool;

impl Tool for OrbitLearningSearchTool {
    fn schema(&self) -> ToolSchema {
        let parameters = vec![
            ToolParam {
                name: "path".to_string(),
                description:
                    "Test this path against every learning's `scope.paths`. A learning matches when any glob entry resolves true."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "tag".to_string(),
                description:
                    "Test this tag against every learning's `scope.tags` (case-insensitive equality)."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "query".to_string(),
                description:
                    "Substring-match this string against `summary` (case-insensitive). Phase-1 has no FTS over body."
                        .to_string(),
                param_type: "string".to_string(),
                required: false,
            },
            ToolParam {
                name: "limit".to_string(),
                description: "Cap on returned rows. Default: unlimited.".to_string(),
                param_type: "integer".to_string(),
                required: false,
            },
        ];
        ToolSchema {
            name: "orbit.learning.search".to_string(),
            description:
                "Search active learnings by path glob OR tag (per §3.3 scope-OR semantics). Returns envelope-only results with a `matched_by` annotation indicating which axis triggered each match."
                    .to_string(),
            parameters,
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningSearch)
    }
}
