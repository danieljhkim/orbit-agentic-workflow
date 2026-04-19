use orbit_exec::{ExecRequest, NoSandbox, StdinMode, run_process};
use orbit_common::types::OrbitError;
use serde_json::{Value, json};

use crate::context::RuntimeHost;

pub(super) fn update_knowledge_graph<H: RuntimeHost + ?Sized>(
    host: &H,
    _input: &Value,
) -> Result<Value, OrbitError> {
    let repo_root = host.repo_root()?;

    let result = run_process(
        &ExecRequest {
            program: "orbit".to_string(),
            args: vec!["graph".to_string(), "update".to_string()],
            current_dir: Some(repo_root),
            timeout_ms: Some(120_000),
            stdin_mode: StdinMode::Null,
            environment_mode: orbit_exec::EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )
    .map_err(|e| OrbitError::Execution(format!("knowledge graph update failed: {e}")))?;

    Ok(json!({
        "success": result.success,
        "exit_code": result.exit_code,
        "stderr": result.stderr.trim_end(),
    }))
}
