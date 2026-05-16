use orbit_common::types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitLearningReindexTool;

impl Tool for OrbitLearningReindexTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.learning.reindex".to_string(),
            description:
                "Rebuild the SQLite envelope index from the YAML source of truth. Returns `{ rebuilt_count }`."
                    .to_string(),
            parameters: Vec::new(),
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::LearningReindex)
    }
}
