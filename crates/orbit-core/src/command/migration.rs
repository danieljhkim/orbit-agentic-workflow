use orbit_store::task_type_migration::{TaskTypeMigrationSummary, migrate_task_types};

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub fn migrate_task_types(
        &self,
        dry_run: bool,
    ) -> Result<TaskTypeMigrationSummary, OrbitError> {
        migrate_task_types(&self.data_root().join("tasks"), dry_run)
    }
}
