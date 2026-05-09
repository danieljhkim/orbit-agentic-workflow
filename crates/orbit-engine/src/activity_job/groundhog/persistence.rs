use chrono::{DateTime, Utc};
use orbit_common::groundhog::{Attempt, Chronicle, FailureReport};
use orbit_common::types::{TaskArtifact, TaskPlan, TaskPlanCheckpoint};
use orbit_tools::ToolContext;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::dispatcher::{DispatchError, V2RuntimeHost};

const CHRONICLE_ARTIFACT_PATH: &str = "artifacts.chronicle";
const STATE_ARTIFACT_PATH: &str = "groundhog/state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct GroundhogRunnerState {
    pub(super) next_snapshot_n: u32,
    pub(super) current: Option<ActiveCheckpointState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ActiveCheckpointState {
    pub(super) checkpoint_index: usize,
    pub(super) checkpoint_id: String,
    pub(super) started_at: DateTime<Utc>,
    pub(super) attempts: Vec<Attempt>,
    pub(super) latest_failure_report: Option<FailureReport>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GroundhogTaskSnapshot {
    pub(super) plan: String,
    pub(super) workspace_path: Option<String>,
}

pub(super) fn persist_runner_artifacts(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    task_id: &str,
    chronicle: &Chronicle,
    state: &GroundhogRunnerState,
    status: Option<&str>,
    comment: Option<String>,
) -> Result<(), DispatchError> {
    let chronicle_json = serde_json::to_string_pretty(chronicle)
        .map_err(|error| DispatchError::GroundhogFailed(format!("serialize chronicle: {error}")))?;
    let state_json = serde_json::to_string_pretty(state)
        .map_err(|error| DispatchError::GroundhogFailed(format!("serialize state: {error}")))?;
    let artifacts = vec![
        TaskArtifact {
            path: CHRONICLE_ARTIFACT_PATH.to_string(),
            content: chronicle_json,
        },
        TaskArtifact {
            path: STATE_ARTIFACT_PATH.to_string(),
            content: state_json,
        },
    ];

    let mut args = json!({
        "id": task_id,
        "artifacts": artifacts
    });
    if let Some(status) = status {
        args["status"] = Value::String(status.to_string());
    }
    if let Some(comment) = comment {
        args["comment"] = Value::String(comment);
    }

    let _ = host
        .run_deterministic(
            "orbit_tool_call",
            &Value::Null,
            &json!({
                "tool_name": "orbit.task.update",
                "args": args
            }),
            tool_ctx.clone(),
        )
        .map_err(|error| {
            DispatchError::GroundhogFailed(format!("persist groundhog artifacts: {error}"))
        })?;
    Ok(())
}

pub(super) fn load_task(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    task_id: &str,
) -> Result<GroundhogTaskSnapshot, DispatchError> {
    let value = host
        .run_deterministic(
            "orbit_tool_call",
            &Value::Null,
            &json!({
                "tool_name": "orbit.task.show",
                "args": { "id": task_id }
            }),
            tool_ctx.clone(),
        )
        .map_err(|error| {
            DispatchError::GroundhogFailed(format!("load task `{task_id}`: {error}"))
        })?;
    serde_json::from_value(value)
        .map_err(|error| DispatchError::GroundhogFailed(format!("parse task `{task_id}`: {error}")))
}

fn load_task_artifacts(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    task_id: &str,
) -> Result<Vec<TaskArtifact>, DispatchError> {
    let value = host
        .run_deterministic(
            "orbit_tool_call",
            &Value::Null,
            &json!({
                "tool_name": "orbit.task.show",
                "args": { "id": task_id, "field": "artifacts" }
            }),
            tool_ctx.clone(),
        )
        .map_err(|error| {
            DispatchError::GroundhogFailed(format!("load artifacts for task `{task_id}`: {error}"))
        })?;
    serde_json::from_value(value).map_err(|error| {
        DispatchError::GroundhogFailed(format!("parse artifacts for task `{task_id}`: {error}"))
    })
}

pub(super) fn load_chronicle(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    task_id: &str,
) -> Result<Chronicle, DispatchError> {
    let artifacts = load_task_artifacts(host, tool_ctx, task_id)?;
    let Some(artifact) = artifacts
        .into_iter()
        .find(|artifact| artifact.path == CHRONICLE_ARTIFACT_PATH)
    else {
        return Ok(Chronicle::new(
            task_id.to_string(),
            format!("{task_id}:plan"),
        ));
    };

    serde_json::from_str(&artifact.content).map_err(|error| {
        DispatchError::GroundhogFailed(format!(
            "parse chronicle artifact for task `{task_id}`: {error}"
        ))
    })
}

pub(super) fn load_state(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    task_id: &str,
) -> Result<GroundhogRunnerState, DispatchError> {
    let artifacts = load_task_artifacts(host, tool_ctx, task_id)?;
    let Some(artifact) = artifacts
        .into_iter()
        .find(|artifact| artifact.path == STATE_ARTIFACT_PATH)
    else {
        return Ok(GroundhogRunnerState::default());
    };

    serde_json::from_str(&artifact.content).map_err(|error| {
        DispatchError::GroundhogFailed(format!(
            "parse runner state artifact for task `{task_id}`: {error}"
        ))
    })
}

pub(super) fn parse_groundhog_plan(
    raw_plan: &str,
    task_id: &str,
) -> Result<TaskPlan, DispatchError> {
    let label = format!("task `{task_id}` plan");
    let plan = TaskPlan::parse(raw_plan, &label)
        .map_err(|error| DispatchError::GroundhogFailed(error.to_string()))?;
    if plan.is_empty() {
        return Err(DispatchError::GroundhogFailed(format!(
            "task `{task_id}` does not contain a structured Groundhog checkpoint plan"
        )));
    }
    Ok(plan)
}

pub(super) fn validate_checkpoint_alignment(
    state: &ActiveCheckpointState,
    plan: &TaskPlan,
    task_id: &str,
) -> Result<(), DispatchError> {
    let checkpoint = plan
        .checkpoints
        .get(state.checkpoint_index)
        .ok_or_else(|| {
            DispatchError::GroundhogFailed(format!(
                "task `{task_id}` current checkpoint index {} is out of bounds",
                state.checkpoint_index
            ))
        })?;
    if checkpoint.id != state.checkpoint_id {
        return Err(DispatchError::GroundhogFailed(format!(
            "task `{task_id}` checkpoint state mismatch: expected `{}`, found `{}`",
            checkpoint.id, state.checkpoint_id
        )));
    }
    Ok(())
}

pub(super) fn effective_attempt_budget(checkpoint: &TaskPlanCheckpoint, fallback: u32) -> u32 {
    checkpoint.attempt_budget.max(fallback).max(1)
}
