use std::io::Write;
use std::time::Instant;

use orbit_types::{ExecutionResult, OrbitError};

use crate::sandbox::Sandbox;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StdinMode {
    #[default]
    Inherit,
    Null,
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct ExecRequest {
    pub program: String,
    pub args: Vec<String>,
    pub timeout_ms: Option<u64>,
    pub stdin_mode: StdinMode,
}

pub fn run_process(
    req: &ExecRequest,
    sandbox: &dyn Sandbox,
) -> Result<ExecutionResult, OrbitError> {
    sandbox.validate(req)?;

    let started = Instant::now();
    let mut child = crate::process::spawn(req)?;
    if let StdinMode::Bytes(bytes) = &req.stdin_mode {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(bytes).map_err(|e| {
                OrbitError::Execution(format!("failed to write process stdin: {e}"))
            })?;
        } else {
            return Err(OrbitError::Execution(
                "stdin requested but no stdin pipe available".to_string(),
            ));
        }
    }
    let (timed_out, output) = crate::timeout::wait_with_optional_timeout(child, req.timeout_ms)?;

    let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if timed_out {
        if !stderr.is_empty() {
            stderr.push('\n');
        }
        stderr.push_str("process timed out");
    }

    Ok(ExecutionResult {
        success: output.status.success() && !timed_out,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr,
        exit_code: output.status.code(),
        duration_ms: started.elapsed().as_millis() as u64,
        output: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::sandbox::NoSandbox;

    use super::*;

    #[test]
    fn captures_stdout() {
        let result = run_process(
            &ExecRequest {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "printf hello".to_string()],
                timeout_ms: Some(1000),
                stdin_mode: StdinMode::Inherit,
            },
            &NoSandbox,
        )
        .expect("process succeeds");

        assert!(result.success);
        assert_eq!(result.stdout, "hello");
    }

    #[test]
    fn supports_stdin_bytes() {
        let result = run_process(
            &ExecRequest {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "cat".to_string()],
                timeout_ms: Some(1000),
                stdin_mode: StdinMode::Bytes(b"hello-stdin".to_vec()),
            },
            &NoSandbox,
        )
        .expect("process succeeds");

        assert!(result.success);
        assert_eq!(result.stdout, "hello-stdin");
    }

    #[test]
    fn enforces_timeout() {
        let result = run_process(
            &ExecRequest {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 1".to_string()],
                timeout_ms: Some(100),
                stdin_mode: StdinMode::Inherit,
            },
            &NoSandbox,
        )
        .expect("process returns timed out result");

        assert!(!result.success);
        assert!(result.stderr.contains("timed out"));
    }
}
