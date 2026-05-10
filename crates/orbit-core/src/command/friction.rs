use orbit_common::types::OrbitError;
use orbit_store::friction_store::{FrictionMigrationSummary, migrate_legacy_friction_tasks};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn migrate_legacy_frictions(&self) -> Result<FrictionMigrationSummary, OrbitError> {
        let tasks = self.list_tasks()?;
        migrate_legacy_friction_tasks(&self.data_root().join("frictions"), &tasks)
    }
}
