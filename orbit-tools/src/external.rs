use std::io::Write;
use std::process::{Command, Stdio};

use orbit_types::{OrbitError, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct ExternalTool {
    pub name: String,
    pub path: String,
    pub description: String,
}

impl Tool for ExternalTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: vec![],
            builtin: false,
        }
    }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let input_json = serde_json::to_string(&input)
            .map_err(|e| OrbitError::Execution(format!("failed to serialize input: {e}")))?;

        let mut child = Command::new(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| OrbitError::Execution(format!("failed to spawn '{}': {e}", self.path)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_json.as_bytes())
                .map_err(|e| OrbitError::Execution(format!("failed to write stdin: {e}")))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| OrbitError::Execution(format!("failed to wait for process: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OrbitError::Execution(format!(
                "tool '{}' exited with {}: {}",
                self.name,
                output.status,
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(stdout.trim()).map_err(|e| {
            OrbitError::Execution(format!(
                "tool '{}' produced invalid JSON output: {e}",
                self.name
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use serde_json::json;
    use tempfile::tempdir;

    use crate::Tool;
    use crate::ToolContext;

    use super::ExternalTool;

    fn create_script(dir: &std::path::Path, name: &str, content: &str) -> String {
        let path = dir.join(name);
        fs::write(&path, content).expect("write script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        path.to_string_lossy().to_string()
    }

    #[test]
    fn external_tool_echoes_json() {
        let dir = tempdir().expect("temp dir");
        let script = create_script(dir.path(), "echo-tool.sh", "#!/bin/sh\ncat\n");

        let tool = ExternalTool {
            name: "echo-tool".to_string(),
            path: script,
            description: "Echoes input".to_string(),
        };

        let input = json!({"hello": "world"});
        let output = tool
            .execute(&ToolContext::default(), input.clone())
            .expect("execute");
        assert_eq!(output, input);
    }

    #[test]
    fn external_tool_transforms_input() {
        let dir = tempdir().expect("temp dir");
        let script = create_script(
            dir.path(),
            "transform.sh",
            "#!/bin/sh\necho '{\"result\": \"ok\"}'\n",
        );

        let tool = ExternalTool {
            name: "transform".to_string(),
            path: script,
            description: "Returns fixed output".to_string(),
        };

        let output = tool
            .execute(&ToolContext::default(), json!({}))
            .expect("execute");
        assert_eq!(output["result"], "ok");
    }

    #[test]
    fn external_tool_failure_returns_error() {
        let dir = tempdir().expect("temp dir");
        let script = create_script(dir.path(), "fail.sh", "#!/bin/sh\necho 'bad' >&2\nexit 1\n");

        let tool = ExternalTool {
            name: "fail-tool".to_string(),
            path: script,
            description: "Always fails".to_string(),
        };

        let result = tool.execute(&ToolContext::default(), json!({}));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("bad"), "error should contain stderr: {err}");
    }

    #[test]
    fn external_tool_schema_is_not_builtin() {
        let tool = ExternalTool {
            name: "test".to_string(),
            path: "/bin/true".to_string(),
            description: "test desc".to_string(),
        };
        let schema = tool.schema();
        assert!(!schema.builtin);
        assert_eq!(schema.name, "test");
    }
}
