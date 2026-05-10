use orbit_common::types::OrbitError;
use orbit_embed::commands::{self as semantic_commands};

pub use orbit_embed::commands::{
    CompanionStatus, ScoreBreakdown, SemanticHit, SemanticInstallParams, SemanticInstallResult,
    SemanticReindexParams, SemanticReindexResult, SemanticRelatedParams, SemanticRelatedResult,
    SemanticSearchParams, SemanticSearchResult, SemanticStatsResult, SemanticUninstallParams,
    SemanticUninstallResult,
};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn semantic_install(
        &self,
        params: SemanticInstallParams,
    ) -> Result<SemanticInstallResult, OrbitError> {
        semantic_commands::install::run(params)
    }

    pub fn semantic_uninstall(
        &self,
        params: SemanticUninstallParams,
    ) -> Result<SemanticUninstallResult, OrbitError> {
        semantic_commands::uninstall::run(params)
    }

    pub fn semantic_reindex(
        &self,
        params: SemanticReindexParams,
    ) -> Result<SemanticReindexResult, OrbitError> {
        let tasks = self.stores().tasks().list()?;
        semantic_commands::reindex::run(&self.stores().semantic_vector, &tasks, params)
    }

    pub fn semantic_stats(&self) -> Result<SemanticStatsResult, OrbitError> {
        let task_ids: Vec<String> = self
            .stores()
            .tasks()
            .list()?
            .into_iter()
            .map(|task| task.id)
            .collect();
        semantic_commands::stats::run(&self.stores().semantic_vector, &task_ids)
    }

    pub fn semantic_search(
        &self,
        params: SemanticSearchParams,
    ) -> Result<SemanticSearchResult, OrbitError> {
        semantic_commands::search::run(&self.stores().semantic_vector, params)
    }

    pub fn semantic_related(
        &self,
        params: SemanticRelatedParams,
    ) -> Result<SemanticRelatedResult, OrbitError> {
        let tasks = self.stores().tasks().list()?;
        semantic_commands::related::run(&self.stores().semantic_vector, &tasks, params)
    }
}
