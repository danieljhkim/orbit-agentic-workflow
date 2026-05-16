use orbit_common::types::OrbitError;

pub use orbit_embed::{
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
        orbit_embed::semantic_install(params)
    }

    pub fn semantic_uninstall(
        &self,
        params: SemanticUninstallParams,
    ) -> Result<SemanticUninstallResult, OrbitError> {
        orbit_embed::semantic_uninstall(params)
    }

    pub fn semantic_reindex(
        &self,
        params: SemanticReindexParams,
    ) -> Result<SemanticReindexResult, OrbitError> {
        let tasks = self.stores().tasks().list()?;
        orbit_embed::semantic_reindex(&self.stores().semantic_vector, &tasks, params)
    }

    pub fn semantic_stats(&self) -> Result<SemanticStatsResult, OrbitError> {
        let task_ids: Vec<String> = self
            .stores()
            .tasks()
            .list()?
            .into_iter()
            .map(|task| task.id)
            .collect();
        orbit_embed::semantic_stats(&self.stores().semantic_vector, &task_ids)
    }

    pub fn semantic_search(
        &self,
        params: SemanticSearchParams,
    ) -> Result<SemanticSearchResult, OrbitError> {
        orbit_embed::semantic_search(&self.stores().semantic_vector, params)
    }

    pub fn semantic_related(
        &self,
        params: SemanticRelatedParams,
    ) -> Result<SemanticRelatedResult, OrbitError> {
        let tasks = self.stores().tasks().list()?;
        orbit_embed::semantic_related(&self.stores().semantic_vector, &tasks, params)
    }
}
