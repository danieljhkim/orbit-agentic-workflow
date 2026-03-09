use orbit_types::{AgentResponseEnvelope, ExecutionResult, OrbitError};
use serde_json::{Deserializer, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResponse {
    pub runtime_key: &'static str,
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
    pub required_env_vars: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentResponseStatus {
    Success,
    Failed,
    Timeout,
}

pub fn parse_and_validate_response(
    exec_result: &ExecutionResult,
) -> Result<(AgentResponseEnvelope, AgentResponseStatus), OrbitError> {
    let stderr_hint = exec_result.stderr.trim();
    if exec_result.stdout.trim().is_empty() {
        if !stderr_hint.is_empty() {
            return Err(OrbitError::Execution(format!(
                "agent did not produce JSON stdout; stderr: {}",
                truncate(stderr_hint, 300)
            )));
        }
        return Err(OrbitError::AgentProtocolViolation(
            "agent stdout is empty".to_string(),
        ));
    }

    let value = match parse_single_json_document(&exec_result.stdout) {
        Ok(value) => value,
        Err(err) => {
            if is_invocation_failure(exec_result) {
                return Err(OrbitError::Execution(format!(
                    "agent did not produce valid JSON stdout; stderr: {}; stdout: {}",
                    truncate(stderr_hint, 300),
                    truncate(exec_result.stdout.trim(), 300),
                )));
            }
            return Err(err);
        }
    };
    let envelope: AgentResponseEnvelope = serde_json::from_value(value).map_err(|error| {
        OrbitError::AgentProtocolViolation(format!("invalid agent response envelope: {error}"))
    })?;

    if envelope.schema_version != 1 {
        return Err(OrbitError::AgentProtocolViolation(format!(
            "unsupported schemaVersion: {}",
            envelope.schema_version
        )));
    }

    let state = match envelope.status.as_str() {
        "success" => AgentResponseStatus::Success,
        "failed" => {
            let Some(error) = &envelope.error else {
                return Err(OrbitError::AgentProtocolViolation(
                    "failed status requires error object".to_string(),
                ));
            };
            if error.code.trim().is_empty() {
                return Err(OrbitError::AgentProtocolViolation(
                    "failed status requires non-empty error.code".to_string(),
                ));
            }
            AgentResponseStatus::Failed
        }
        "timeout" => AgentResponseStatus::Timeout,
        other => {
            return Err(OrbitError::AgentProtocolViolation(format!(
                "unknown status: {other}"
            )));
        }
    };

    validate_exit_alignment(exec_result, &envelope)?;

    Ok((envelope, state))
}

pub fn is_timeout(exec_result: &ExecutionResult) -> bool {
    !exec_result.success && exec_result.stderr.contains("process timed out")
}

fn parse_single_json_document(stdout: &str) -> Result<Value, OrbitError> {
    let mut stream = Deserializer::from_str(stdout).into_iter::<Value>();

    let Some(first) = stream.next() else {
        return Err(OrbitError::AgentProtocolViolation(
            "stdout does not contain a JSON document".to_string(),
        ));
    };

    let first = first.map_err(|error| {
        OrbitError::AgentProtocolViolation(format!("stdout is not valid JSON: {error}"))
    })?;

    if stream.next().is_some() {
        return Err(OrbitError::AgentProtocolViolation(
            "stdout contains multiple JSON documents".to_string(),
        ));
    }

    Ok(first)
}

fn validate_exit_alignment(
    exec_result: &ExecutionResult,
    envelope: &AgentResponseEnvelope,
) -> Result<(), OrbitError> {
    let timed_out = is_timeout(exec_result);

    if timed_out && envelope.status != "timeout" {
        return Err(OrbitError::AgentProtocolViolation(
            "timeout process must report status=timeout".to_string(),
        ));
    }

    if timed_out {
        return Ok(());
    }

    let exit_code = exec_result.exit_code.unwrap_or(1);
    if exit_code == 0 && envelope.status != "success" {
        return Err(OrbitError::AgentProtocolViolation(
            "exit_code=0 must report status=success".to_string(),
        ));
    }
    if exit_code != 0 && envelope.status == "success" {
        return Err(OrbitError::AgentProtocolViolation(
            "non-zero exit code cannot report status=success".to_string(),
        ));
    }

    Ok(())
}

fn is_invocation_failure(exec_result: &ExecutionResult) -> bool {
    exec_result.exit_code.unwrap_or(1) != 0 && !exec_result.stderr.trim().is_empty()
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_string();
    }
    let mut output = value[..max_len].to_string();
    output.push_str("...");
    output
}
