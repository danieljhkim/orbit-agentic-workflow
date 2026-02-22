use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use orbit_types::{ExecutionResult, OrbitError};
use wait_timeout::ChildExt;

#[derive(Debug, Clone)]
pub struct ExecRequest {
    pub program: String,
    pub args: Vec<String>,
    pub timeout_ms: Option<u64>,
}

pub trait Sandbox {
    fn validate(&self, req: &ExecRequest) -> Result<(), OrbitError>;
}

#[derive(Debug, Default)]
pub struct NoSandbox;

impl Sandbox for NoSandbox {
    fn validate(&self, _req: &ExecRequest) -> Result<(), OrbitError> {
        Ok(())
    }
}

pub fn run_process(
    req: &ExecRequest,
    sandbox: &dyn Sandbox,
) -> Result<ExecutionResult, OrbitError> {
    sandbox.validate(req)?;

    let started = Instant::now();
    let mut child = Command::new(&req.program)
        .args(&req.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| OrbitError::Execution(format!("failed to spawn `{}`: {e}", req.program)))?;

    let (timed_out, output) = match req.timeout_ms {
        Some(timeout_ms) => {
            let timeout = Duration::from_millis(timeout_ms);
            match child
                .wait_timeout(timeout)
                .map_err(|e| OrbitError::Execution(format!("wait timeout error: {e}")))?
            {
                Some(_) => (
                    false,
                    child.wait_with_output().map_err(|e| {
                        OrbitError::Execution(format!("failed waiting for output: {e}"))
                    })?,
                ),
                None => {
                    child.kill().map_err(|e| {
                        OrbitError::Execution(format!("failed to kill timed out process: {e}"))
                    })?;
                    (
                        true,
                        child.wait_with_output().map_err(|e| {
                            OrbitError::Execution(format!(
                                "failed waiting for timed out process output: {e}"
                            ))
                        })?,
                    )
                }
            }
        }
        None => (
            false,
            child.wait_with_output().map_err(|e| {
                OrbitError::Execution(format!("failed waiting for process output: {e}"))
            })?,
        ),
    };

    let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if timed_out {
        if !stderr.is_empty() {
            stderr.push('\n');
        }
        stderr.push_str("process timed out");
    }

    let result = ExecutionResult {
        success: output.status.success() && !timed_out,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr,
        exit_code: output.status.code(),
        duration_ms: started.elapsed().as_millis() as u64,
        output: None,
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_stdout() {
        let result = run_process(
            &ExecRequest {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "printf hello".to_string()],
                timeout_ms: Some(1000),
            },
            &NoSandbox,
        )
        .expect("process succeeds");

        assert!(result.success);
        assert_eq!(result.stdout, "hello");
    }

    #[test]
    fn enforces_timeout() {
        let result = run_process(
            &ExecRequest {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 1".to_string()],
                timeout_ms: Some(100),
            },
            &NoSandbox,
        )
        .expect("process returns timed out result");

        assert!(!result.success);
        assert!(result.stderr.contains("timed out"));
    }
}
