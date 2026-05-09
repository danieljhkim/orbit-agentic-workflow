use std::fs;
use std::path::{Path, PathBuf};

use orbit_common::groundhog::FailureReport;
use orbit_common::types::{ExecutionResult, TaskPlanCheckpoint, TaskPlanSuccessCriterion};
use orbit_tools::ToolContext;
use serde_json::{Value, json};

use super::super::dispatcher::{DispatchError, V2RuntimeHost};

const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 300_000;

pub(super) fn verify_checkpoint(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    workspace_path: &Path,
    checkpoint: &TaskPlanCheckpoint,
) -> Result<VerifierOutcome, DispatchError> {
    for criterion in &checkpoint.success_criteria {
        match criterion {
            TaskPlanSuccessCriterion::Command {
                command,
                expect_exit,
            } => {
                let result = run_workspace_command(host, tool_ctx, workspace_path, command)?;
                let actual = result.exit_code.unwrap_or(-1);
                if actual != *expect_exit {
                    let detail = truncate_for_failure_report(format!(
                        "command `{}` exited {} (expected {})\nstdout:\n{}\nstderr:\n{}",
                        command, actual, expect_exit, result.stdout, result.stderr
                    ));
                    return Ok(VerifierOutcome {
                        failure_report: Some(FailureReport {
                            what_tried: format!(
                                "verified command criterion for checkpoint `{}`",
                                checkpoint.id
                            ),
                            what_happened: detail,
                            next_attempt_plan:
                                "Fix the failing verifier condition before emitting checkpoint_success again."
                                    .to_string(),
                        }),
                    });
                }
            }
            TaskPlanSuccessCriterion::FileExists { path } => {
                let resolved = resolve_checkpoint_path(workspace_path, path);
                if !resolved.exists() {
                    return Ok(VerifierOutcome {
                        failure_report: Some(FailureReport {
                            what_tried: format!(
                                "verified file_exists criterion for checkpoint `{}`",
                                checkpoint.id
                            ),
                            what_happened: format!("required file `{}` does not exist", path),
                            next_attempt_plan:
                                "Create the expected file before emitting checkpoint_success again."
                                    .to_string(),
                        }),
                    });
                }
            }
            TaskPlanSuccessCriterion::FileContains { path, pattern } => {
                let resolved = resolve_checkpoint_path(workspace_path, path);
                let contents = fs::read_to_string(&resolved).map_err(|error| {
                    DispatchError::GroundhogFailed(format!(
                        "read verifier file {}: {error}",
                        resolved.display()
                    ))
                })?;
                if !contents.contains(pattern) {
                    return Ok(VerifierOutcome {
                        failure_report: Some(FailureReport {
                            what_tried: format!(
                                "verified file_contains criterion for checkpoint `{}`",
                                checkpoint.id
                            ),
                            what_happened: format!(
                                "file `{}` does not contain required pattern `{}`",
                                path, pattern
                            ),
                            next_attempt_plan:
                                "Update the file so the required pattern is present before emitting checkpoint_success again."
                                    .to_string(),
                        }),
                    });
                }
            }
            TaskPlanSuccessCriterion::Semantic { .. } => {}
        }
    }

    Ok(VerifierOutcome {
        failure_report: None,
    })
}

fn run_workspace_command(
    host: &dyn V2RuntimeHost,
    tool_ctx: &ToolContext,
    workspace_path: &Path,
    command: &str,
) -> Result<ExecutionResult, DispatchError> {
    let cwd = shell_single_quote(&workspace_path.display().to_string());
    let script = format!("cd {cwd} && {command}");
    let value = host
        .run_deterministic(
            "orbit_tool_call",
            &Value::Null,
            &json!({
                "tool_name": "proc.spawn",
                "args": {
                    "program": "sh",
                    "args": ["-lc", script],
                    "timeout_ms": DEFAULT_COMMAND_TIMEOUT_MS
                }
            }),
            tool_ctx.clone(),
        )
        .map_err(|error| {
            DispatchError::GroundhogFailed(format!("proc.spawn verifier call: {error}"))
        })?;
    serde_json::from_value(value).map_err(|error| {
        DispatchError::GroundhogFailed(format!("parse proc.spawn result: {error}"))
    })
}

fn resolve_checkpoint_path(workspace_path: &Path, raw: &str) -> PathBuf {
    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        workspace_path.join(candidate)
    }
}

fn truncate_for_failure_report(text: String) -> String {
    const MAX_LEN: usize = 4000;
    if text.len() <= MAX_LEN {
        text
    } else {
        format!("{}...[truncated]", &text[..MAX_LEN])
    }
}

fn shell_single_quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\"'\"'"))
}

#[derive(Debug, Clone)]
pub(super) struct VerifierOutcome {
    pub(super) failure_report: Option<FailureReport>,
}
