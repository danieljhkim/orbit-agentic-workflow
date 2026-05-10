use orbit_common::types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{OrbitBuiltinAction, Tool, ToolContext};

pub struct OrbitFrictionStatsTool;

impl Tool for OrbitFrictionStatsTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.friction.stats".to_string(),
            description: "Compute friction rates from .orbit/frictions/ and the task store"
                .to_string(),
            parameters: Vec::new(),
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::FrictionStats)
    }
}
