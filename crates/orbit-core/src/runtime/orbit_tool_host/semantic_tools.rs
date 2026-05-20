use orbit_common::types::{OrbitError, optional_string_alias};
use orbit_search::{SemanticInstallParams, SemanticReindexParams, SemanticUninstallParams};
use serde_json::Value;

use crate::OrbitRuntime;

use super::input::optional_bool_alias;

pub(super) fn install(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    to_json(runtime.semantic_install(SemanticInstallParams {
        model: optional_string_alias(&input, &["model", "embedding_model", "embeddingModel"])?,
        force: optional_bool_alias(&input, &["force"])?.unwrap_or(false),
    })?)
}

pub(super) fn uninstall(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    to_json(runtime.semantic_uninstall(SemanticUninstallParams {
        model: optional_string_alias(&input, &["model", "embedding_model", "embeddingModel"])?,
        all: optional_bool_alias(&input, &["all"])?.unwrap_or(false),
    })?)
}

pub(super) fn stats(runtime: &OrbitRuntime) -> Result<Value, OrbitError> {
    to_json(runtime.semantic_stats()?)
}

pub(super) fn index(runtime: &OrbitRuntime, input: Value) -> Result<Value, OrbitError> {
    to_json(runtime.semantic_reindex(SemanticReindexParams {
        model: optional_string_alias(&input, &["model", "embedding_model", "embeddingModel"])?,
        force: optional_bool_alias(&input, &["force"])?.unwrap_or(false),
    })?)
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Value, OrbitError> {
    serde_json::to_value(value)
        .map_err(|error| OrbitError::Execution(format!("serialize semantic result: {error}")))
}
