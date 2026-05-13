use orbit_common::types::{AuditEventStatus, Role, audit_execution_id};
use orbit_engine::DispatchError;
use orbit_store::AuditEventInsertParams;
use orbit_tools::ToolContext;
use serde_json::Value;

use crate::OrbitRuntime;

pub(super) fn validate_bundles(action: &str, input: &Value) -> Result<Value, DispatchError> {
    let bundles_raw = input
        .get("bundles")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: "`bundles` must be an array".to_string(),
        })?;
    let max_bundle_size = input
        .get("max_bundle_size")
        .and_then(Value::as_u64)
        .unwrap_or(5) as usize;
    let known: std::collections::BTreeSet<String> = input
        .get("known_task_ids")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default();

    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut violations: Vec<String> = Vec::new();
    let mut bundles: Vec<Vec<String>> = Vec::with_capacity(bundles_raw.len());
    for (idx, bundle) in bundles_raw.iter().enumerate() {
        let items = bundle
            .as_array()
            .ok_or_else(|| DispatchError::DeterministicActionFailed {
                action: action.to_string(),
                message: format!("bundle[{idx}] is not an array"),
            })?;
        if items.len() > max_bundle_size {
            violations.push(format!(
                "bundle[{idx}] size {} exceeds max_bundle_size {}",
                items.len(),
                max_bundle_size
            ));
        }
        let mut bundle_ids: Vec<String> = Vec::with_capacity(items.len());
        for item in items {
            let id = item
                .as_str()
                .ok_or_else(|| DispatchError::DeterministicActionFailed {
                    action: action.to_string(),
                    message: format!("bundle[{idx}] contains a non-string task_id"),
                })?;
            if !known.is_empty() && !known.contains(id) {
                violations.push(format!("bundle[{idx}] references unknown task_id {id}"));
            }
            if !seen.insert(id.to_string()) {
                violations.push(format!("task_id {id} appears in more than one bundle"));
            }
            bundle_ids.push(id.to_string());
        }
        bundles.push(bundle_ids);
    }
    if !violations.is_empty() {
        return Err(DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: format!("invalid bundles: {}", violations.join("; ")),
        });
    }
    Ok(serde_json::json!({
        "bundles": bundles,
        "bundle_count": bundles.len(),
    }))
}

pub(super) fn invoke_and_wait(
    runtime: &OrbitRuntime,
    action: &str,
    input: &Value,
    tool_context: ToolContext,
) -> Result<Value, DispatchError> {
    let job_name = input
        .get("job_name")
        .and_then(Value::as_str)
        .ok_or_else(|| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: "missing `job_name`".to_string(),
        })?
        .to_string();
    let run_input = input
        .get("run_input")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    let mut invoke_args = serde_json::Map::new();
    invoke_args.insert("job_name".to_string(), Value::String(job_name.clone()));
    invoke_args.insert("input".to_string(), run_input);
    if let Some(priority) = input.get("priority").cloned() {
        invoke_args.insert("priority".to_string(), priority);
    }

    let invoke_ctx = tool_context.clone();
    let invoke_output = runtime
        .run_tool_with_context_and_role(
            "orbit.pipeline.invoke",
            Value::Object(invoke_args),
            Role::Admin,
            invoke_ctx,
        )
        .map_err(|err| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: format!("pipeline.invoke failed: {err}"),
        })?;

    let run_id = invoke_output
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: "pipeline.invoke returned no run_id".to_string(),
        })?
        .to_string();

    let mut wait_args = serde_json::Map::new();
    wait_args.insert(
        "run_ids".to_string(),
        Value::Array(vec![Value::String(run_id.clone())]),
    );
    if let Some(timeout) = input.get("timeout_seconds").cloned() {
        wait_args.insert("timeout_seconds".to_string(), timeout);
    }
    if let Some(poll) = input.get("poll_interval_seconds").cloned() {
        wait_args.insert("poll_interval_seconds".to_string(), poll);
    }

    let wait_output = runtime
        .run_tool_with_context_and_role(
            "orbit.pipeline.wait",
            Value::Object(wait_args),
            Role::Admin,
            tool_context,
        )
        .map_err(|err| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: format!("pipeline.wait failed: {err}"),
        })?;

    let first = wait_output
        .get("results")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .cloned()
        .unwrap_or_else(|| {
            serde_json::json!({
                "run_id": run_id,
                "status": "pending",
            })
        });
    Ok(first)
}

pub(super) fn pipeline_wait(
    runtime: &OrbitRuntime,
    action: &str,
    input: &Value,
    tool_context: ToolContext,
) -> Result<Value, DispatchError> {
    let run_ids =
        input
            .get("run_ids")
            .cloned()
            .ok_or_else(|| DispatchError::DeterministicActionFailed {
                action: action.to_string(),
                message: "missing `run_ids`".to_string(),
            })?;

    let mut wait_args = serde_json::Map::new();
    wait_args.insert("run_ids".to_string(), run_ids);
    if let Some(timeout) = input.get("timeout_seconds").cloned() {
        wait_args.insert("timeout_seconds".to_string(), timeout);
    }
    if let Some(poll) = input.get("poll_interval_seconds").cloned() {
        wait_args.insert("poll_interval_seconds".to_string(), poll);
    }

    runtime
        .run_tool_with_context_and_role(
            "orbit.pipeline.wait",
            Value::Object(wait_args),
            Role::Admin,
            tool_context,
        )
        .map_err(|err| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: format!("pipeline.wait failed: {err}"),
        })
}

pub(super) fn pipeline_success_guard(action: &str, input: &Value) -> Result<Value, DispatchError> {
    let context = input
        .get("context")
        .and_then(Value::as_str)
        .unwrap_or("pipeline child run");
    let mut checked_count = 0usize;
    let mut failures = Vec::new();

    if let Some(result) = input.get("result")
        && !result.is_null()
    {
        checked_count += 1;
        if let Some(failure) = pipeline_wait_entry_failure("result", result) {
            failures.push(failure);
        }
    }

    if let Some(results) = input.get("results")
        && !results.is_null()
    {
        let entries =
            results
                .as_array()
                .ok_or_else(|| DispatchError::DeterministicActionFailed {
                    action: action.to_string(),
                    message: "`results` must be an array".to_string(),
                })?;
        for (idx, entry) in entries.iter().enumerate() {
            checked_count += 1;
            if let Some(failure) = pipeline_wait_entry_failure(&format!("results[{idx}]"), entry) {
                failures.push(failure);
            }
        }
    }

    if checked_count == 0 {
        return Err(DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: "expected `result` or `results` to check".to_string(),
        });
    }

    if !failures.is_empty() {
        return Err(DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: format!("{context} did not succeed: {}", failures.join("; ")),
        });
    }

    Ok(serde_json::json!({
        "succeeded": true,
        "checked_count": checked_count,
    }))
}

fn pipeline_wait_entry_failure(label: &str, entry: &Value) -> Option<String> {
    let Some(status) = entry.get("status").and_then(Value::as_str) else {
        return Some(format!("{label} missing string status"));
    };
    if status == "succeeded" {
        return None;
    }

    let run_id = entry
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or("<unknown>");
    let error = entry
        .get("error")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    Some(match error {
        Some(error) => format!("{label} run {run_id} status {status}: {error}"),
        None => format!("{label} run {run_id} status {status}"),
    })
}

pub(super) fn gate_starvation_fail(
    runtime: &OrbitRuntime,
    action: &str,
    input: &Value,
) -> Result<Value, DispatchError> {
    let task_ids_vec: Vec<String> = input
        .get("task_ids")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let conflicts = input
        .get("conflicts")
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    let max_wait_seconds = input.get("max_wait_seconds").and_then(Value::as_f64);
    let conflicting_files: Vec<String> = conflicts
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    entry
                        .get("file")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();

    let payload = serde_json::json!({
        "task_ids": task_ids_vec,
        "conflicting_files": conflicting_files,
        "conflicts": conflicts,
        "max_wait_seconds": max_wait_seconds,
    });

    let execution_id = audit_execution_id("audit-gate-starvation");
    let working_directory = runtime.paths().repo_root.to_string_lossy().into_owned();
    runtime
        .record_audit_event(&AuditEventInsertParams {
            execution_id,
            command: "gate.starvation".to_string(),
            subcommand: None,
            tool_name: None,
            target_type: Some("task_bundle".to_string()),
            target_id: task_ids_vec.first().cloned(),
            role: "admin".to_string(),
            status: AuditEventStatus::Failure,
            exit_code: 1,
            duration_ms: 0,
            working_directory,
            arguments_json: Some(serde_json::to_string(&payload).map_err(|error| {
                DispatchError::DeterministicActionFailed {
                    action: action.to_string(),
                    message: format!("serialize gate.starvation payload: {error}"),
                }
            })?),
            stdout_truncated: None,
            stderr_truncated: None,
            error_message: Some("gate.starvation".to_string()),
            host: std::env::var("HOSTNAME").ok(),
            pid: std::process::id(),
            session_id: None,
            task_id: task_ids_vec.first().cloned(),
            job_run_id: None,
            activity_id: None,
            step_index: None,
        })
        .map_err(|err| DispatchError::DeterministicActionFailed {
            action: action.to_string(),
            message: format!("record gate.starvation audit: {err}"),
        })?;

    Err(DispatchError::DeterministicActionFailed {
        action: action.to_string(),
        message: format!(
            "gate.starvation: admission window never opened for bundle {:?} \
             (conflicting_files={:?}, max_wait_seconds={:?})",
            task_ids_vec, conflicting_files, max_wait_seconds
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn action_failure_message(err: DispatchError) -> String {
        match err {
            DispatchError::DeterministicActionFailed { action, message } => {
                assert_eq!(action, "pipeline_success_guard");
                message
            }
            other => panic!("expected deterministic action failure, got {other}"),
        }
    }

    #[test]
    fn pipeline_success_guard_accepts_succeeded_result() {
        let output = pipeline_success_guard(
            "pipeline_success_guard",
            &json!({
                "result": {
                    "run_id": "jrun-ok",
                    "status": "succeeded"
                }
            }),
        )
        .expect("succeeded result should pass");

        assert_eq!(output["succeeded"], json!(true));
        assert_eq!(output["checked_count"], json!(1));
    }

    #[test]
    fn pipeline_success_guard_rejects_failed_result() {
        let err = pipeline_success_guard(
            "pipeline_success_guard",
            &json!({
                "context": "task gate child",
                "result": {
                    "run_id": "jrun-failed",
                    "status": "failed",
                    "error": "implementation failed"
                }
            }),
        )
        .expect_err("failed child run should fail the guard");

        let message = action_failure_message(err);
        assert!(message.contains("task gate child did not succeed"));
        assert!(message.contains("jrun-failed"));
        assert!(message.contains("status failed"));
        assert!(message.contains("implementation failed"));
    }

    #[test]
    fn pipeline_success_guard_rejects_mixed_results() {
        let err = pipeline_success_guard(
            "pipeline_success_guard",
            &json!({
                "results": [
                    {
                        "run_id": "jrun-ok",
                        "status": "succeeded"
                    },
                    {
                        "run_id": "jrun-cancelled",
                        "status": "cancelled"
                    },
                    null
                ]
            }),
        )
        .expect_err("any non-succeeded result should fail the guard");

        let message = action_failure_message(err);
        assert!(message.contains("results[1] run jrun-cancelled status cancelled"));
        assert!(message.contains("results[2] missing string status"));
    }
}
