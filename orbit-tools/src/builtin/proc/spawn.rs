use orbit_exec::{ExecRequest, NoSandbox, run_process};
use orbit_types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct ProcSpawnTool;

impl Tool for ProcSpawnTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "proc.spawn".to_string(),
            description: "Spawn a process with timeout and capture output".to_string(),
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let program = input
            .get("program")
            .and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `program`".to_string()))?
            .to_string();

        let args = input
            .get("args")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let timeout_ms = input.get("timeout_ms").and_then(Value::as_u64);

        let exec_result = run_process(
            &ExecRequest {
                program,
                args,
                timeout_ms,
            },
            &NoSandbox,
        )?;

        serde_json::to_value(exec_result)
            .map_err(|e| OrbitError::Execution(format!("serialize exec result: {e}")))
    }
}
