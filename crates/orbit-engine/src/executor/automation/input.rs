use std::path::PathBuf;

use orbit_common::types::OrbitError;
use serde_json::Value;

pub(super) fn required_input_string<'a>(
    input: &'a Value,
    key: &str,
) -> Result<&'a str, OrbitError> {
    input
        .as_object()
        .and_then(|map| map.get(key))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| OrbitError::InvalidInput(format!("missing required input.{key}")))
}

pub(super) fn required_job_run_id<'a>(
    input: &'a Value,
    activity: &str,
) -> Result<&'a str, OrbitError> {
    let Some(map) = input.as_object() else {
        return Err(OrbitError::InvalidInput(format!(
            "{activity} requires input.job_run_id, input.run_id, or input.batch_id"
        )));
    };

    for key in ["job_run_id", "run_id", "batch_id"] {
        if let Some(value) = map
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(value);
        }
    }

    Err(OrbitError::InvalidInput(format!(
        "{activity} requires input.job_run_id, input.run_id, or input.batch_id"
    )))
}

pub(super) fn input_string_field(input: &Value, key: &str) -> Option<String> {
    input
        .as_object()
        .and_then(|map| map.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn canonicalize_existing_dir(
    raw: &str,
    field_name: &str,
) -> Result<PathBuf, OrbitError> {
    let path = PathBuf::from(raw);
    if !path.exists() {
        return Err(OrbitError::InvalidInput(format!(
            "{field_name} does not exist: {raw}"
        )));
    }
    if !path.is_dir() {
        return Err(OrbitError::InvalidInput(format!(
            "{field_name} is not a directory: {raw}"
        )));
    }
    path.canonicalize().map_err(|error| {
        OrbitError::InvalidInput(format!(
            "failed to canonicalize {field_name} '{raw}': {error}"
        ))
    })
}

pub(super) fn json_number_to_string(value: &Value) -> Option<String> {
    value
        .as_i64()
        .map(|number| number.to_string())
        .or_else(|| value.as_u64().map(|number| number.to_string()))
        .or_else(|| value.as_str().map(ToOwned::to_owned))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use crate::template::TemplateContext;

    use super::*;

    #[test]
    fn required_job_run_id_prefers_job_run_id_over_run_id_and_batch_id() {
        let input = json!({
            "job_run_id": "job-run",
            "run_id": "run",
            "batch_id": "batch",
        });

        assert_eq!(required_job_run_id(&input, "pr_open").unwrap(), "job-run");
    }

    #[test]
    fn required_job_run_id_falls_back_to_run_id_before_batch_id() {
        let input = json!({
            "job_run_id": "",
            "run_id": "run",
            "batch_id": "batch",
        });

        assert_eq!(required_job_run_id(&input, "pr_open").unwrap(), "run");
    }

    #[test]
    fn required_job_run_id_accepts_legacy_batch_id() {
        let input = json!({
            "batch_id": "legacy-batch",
        });

        assert_eq!(
            required_job_run_id(&input, "pr_open").unwrap(),
            "legacy-batch"
        );
    }

    #[test]
    fn required_job_run_id_names_batch_id_in_missing_key_error() {
        let error = required_job_run_id(&json!({}), "pr_open").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("requires input.job_run_id, input.run_id, or input.batch_id")
        );
    }

    #[test]
    fn legacy_batch_id_template_output_resolves_as_job_run_id() {
        let mut steps = HashMap::new();
        steps.insert(
            "worktree".to_string(),
            json!({
                "output": {
                    "job_run_id": "jrun-legacy",
                    "batch_id": "jrun-legacy",
                }
            }),
        );
        let context = TemplateContext {
            steps,
            ..TemplateContext::default()
        };
        let batch_id =
            crate::template::render("{{ steps.worktree.output.batch_id }}", &context).unwrap();
        let input = json!({
            "batch_id": batch_id,
        });

        assert_eq!(
            required_job_run_id(&input, "pr_open").unwrap(),
            "jrun-legacy"
        );
    }
}
