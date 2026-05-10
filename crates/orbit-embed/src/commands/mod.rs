//! Canonical command surface for semantic-search operations.
//!
//! Tool adapters and CLI delegates parse request envelopes, call into these
//! commands, and shape the returned typed results for their transport. The
//! top-level `OrbitRuntime` exposes thin delegates that build the runtime's
//! shared state into these calls.

pub mod install;
pub mod reindex;
pub mod related;
pub mod search;
pub mod stats;
pub mod uninstall;

pub use install::{SemanticInstallParams, SemanticInstallResult};
pub use reindex::{SemanticReindexParams, SemanticReindexResult};
pub use related::{SemanticRelatedParams, SemanticRelatedResult};
pub use search::{ScoreBreakdown, SemanticHit, SemanticSearchParams, SemanticSearchResult};
pub use stats::{CompanionStatus, SemanticStatsResult};
pub use uninstall::{SemanticUninstallParams, SemanticUninstallResult};

use std::fs;

use orbit_common::types::OrbitError;

use crate::{CompanionPaths, ModelSpec, default_model};

pub(crate) const DEFAULT_RELEASE_BASE_URL: &str =
    "https://github.com/danieljhkim/orbit/releases/latest/download";

pub(crate) fn parse_model(model: Option<&str>) -> Result<ModelSpec, OrbitError> {
    match model {
        Some(value) => ModelSpec::parse(value),
        None => Ok(default_model()),
    }
}

pub(crate) fn resolve_query_model(model: Option<&str>) -> Result<ModelSpec, OrbitError> {
    if model.is_some() {
        return parse_model(model);
    }
    let active = CompanionPaths::default_under_home()
        .ok()
        .and_then(|paths| active_model(&paths));
    parse_model(active.as_deref())
}

pub(crate) fn active_model(paths: &CompanionPaths) -> Option<String> {
    fs::read_to_string(&paths.active_model_path)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn remove_file_if_exists(path: &std::path::Path) -> Result<bool, OrbitError> {
    if path.exists() {
        fs::remove_file(path).map_err(|error| OrbitError::Io(error.to_string()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}
