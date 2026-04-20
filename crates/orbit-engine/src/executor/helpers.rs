//! Shared helpers used by the v1-style executors (cli_command, automation).
//!
//! These used to live in `activity_runner.rs` which has been retired; the
//! surviving executors are still registered in the [`super::registry::ActivityExecutorRegistry`]
//! and need the schema/template helpers at runtime.

use orbit_common::types::{Activity, OrbitError};
use orbit_store::validate_instance_against_schema;
use serde_json::Value;

use crate::context::{ExecutionContext, input_workspace_path};
use crate::template::TemplateContext;

pub(crate) fn execution_template_context_with_env(
    execution: &ExecutionContext,
    env_pairs: Vec<(String, String)>,
) -> TemplateContext {
    let env = env_pairs
        .into_iter()
        .collect::<std::collections::HashMap<_, _>>();

    TemplateContext {
        input: execution.input.clone(),
        env,
        workspace_path: execution
            .activity
            .workspace_path
            .clone()
            .or_else(|| input_workspace_path(&execution.input)),
        item: None,
        iteration: None,
        steps: execution.steps_outputs.clone(),
    }
}

pub fn validate_activity_output_schema(
    activity: &Activity,
    output: &Value,
) -> Result<(), OrbitError> {
    let context = format!(
        "activity '{}' output does not match output schema",
        activity.id
    );
    validate_instance_against_schema(&activity.output_schema_json, output, &context)
}
