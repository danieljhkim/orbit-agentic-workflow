use serde_json::Value;

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub fn execute_tool_command(&self, name: &str, input: Value) -> Result<Value, OrbitError> {
        self.run_tool(name, input)
    }
}
