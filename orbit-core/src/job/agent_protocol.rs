use std::path::Path;

use orbit_types::{AgentResponseEnvelope, ExecutionResult, JobRunState, JobTargetType, OrbitError};
use serde_json::{Deserializer, Value};

#[derive(Debug, Clone)]
pub struct AgentInvocation {
    pub program: String,
    pub args: Vec<String>,
}

pub fn build_invocation(
    agent_cli: &str,
    target_type: JobTargetType,
    target_id: &str,
) -> Result<AgentInvocation, OrbitError> {
    let provider = provider_key(agent_cli);

    let args = match provider.as_str() {
        // Keep provider-specific mappers explicit to avoid hidden command drift.
        "claude" => default_scheduled_args(target_type, target_id),
        "codex" => default_scheduled_args(target_type, target_id),
        "mock-agent" => default_scheduled_args(target_type, target_id),
        _ => {
            return Err(OrbitError::UnsupportedAgentProvider(provider));
        }
    };

    Ok(AgentInvocation {
        program: agent_cli.to_string(),
        args,
    })
}

pub fn parse_and_validate_response(
    exec_result: &ExecutionResult,
) -> Result<(AgentResponseEnvelope, JobRunState), OrbitError> {
    if exec_result.stdout.trim().is_empty() {
        return Err(OrbitError::AgentProtocolViolation(
            "agent stdout is empty".to_string(),
        ));
    }

    let value = parse_single_json_document(&exec_result.stdout)?;
    let envelope: AgentResponseEnvelope = serde_json::from_value(value).map_err(|e| {
        OrbitError::AgentProtocolViolation(format!("invalid agent response envelope: {e}"))
    })?;

    if envelope.schema_version != 1 {
        return Err(OrbitError::AgentProtocolViolation(format!(
            "unsupported schemaVersion: {}",
            envelope.schema_version
        )));
    }

    let state = match envelope.status.as_str() {
        "success" => JobRunState::Success,
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
            JobRunState::Failed
        }
        "timeout" => JobRunState::Timeout,
        other => {
            return Err(OrbitError::AgentProtocolViolation(format!(
                "unknown status: {other}"
            )));
        }
    };

    validate_exit_alignment(exec_result, &envelope)?;

    Ok((envelope, state))
}

fn parse_single_json_document(stdout: &str) -> Result<Value, OrbitError> {
    let mut stream = Deserializer::from_str(stdout).into_iter::<Value>();

    let Some(first) = stream.next() else {
        return Err(OrbitError::AgentProtocolViolation(
            "stdout does not contain a JSON document".to_string(),
        ));
    };

    let first = first.map_err(|e| {
        OrbitError::AgentProtocolViolation(format!("stdout is not valid JSON: {e}"))
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

pub fn is_timeout(exec_result: &ExecutionResult) -> bool {
    !exec_result.success && exec_result.stderr.contains("process timed out")
}

fn default_scheduled_args(target_type: JobTargetType, target_id: &str) -> Vec<String> {
    vec![
        "run".to_string(),
        "--target-type".to_string(),
        target_type.to_string(),
        "--target-id".to_string(),
        target_id.to_string(),
        "--mode".to_string(),
        "scheduled".to_string(),
        "--output".to_string(),
        "json".to_string(),
    ]
}

fn provider_key(agent_cli: &str) -> String {
    Path::new(agent_cli)
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| agent_cli.to_ascii_lowercase())
}
