use orbit_common::types::{OrbitError, optional_string_alias, optional_u32_alias, required_string};
use orbit_embed::commands::{SemanticRelatedParams, SemanticSearchParams};
use serde_json::Value;

use crate::OrbitRuntime;

pub(super) fn search(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    let result = runtime.semantic_search(SemanticSearchParams {
        query: required_string(&input, &["query"], "query")?,
        limit: optional_limit(&input)?.unwrap_or(10),
        field: optional_string_alias(&input, &["field"])?,
        kind: optional_string_alias(&input, &["kind"])?,
        model: optional_string_alias(
            &input,
            &["embedding_model", "embeddingModel", "embedding-model"],
        )?,
    })?;
    serde_json::to_value(result).map_err(|error| {
        OrbitError::Execution(format!("serialize semantic search result: {error}"))
    })
}

pub(super) fn related(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    let result = runtime.semantic_related(SemanticRelatedParams {
        task_id: required_string(&input, &["id", "task_id", "taskId", "task-id"], "id")?,
        limit: optional_limit(&input)?.unwrap_or(10),
        model: optional_string_alias(
            &input,
            &["embedding_model", "embeddingModel", "embedding-model"],
        )?,
    })?;
    serde_json::to_value(result).map_err(|error| {
        OrbitError::Execution(format!("serialize semantic related result: {error}"))
    })
}

fn optional_limit(input: &Value) -> Result<Option<usize>, OrbitError> {
    optional_u32_alias(input, &["limit"]).map(|value| value.map(|limit| limit as usize))
}
