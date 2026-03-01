use std::sync::Arc;

use orbit_policy::PolicyEngine;
use orbit_store::Store;
use orbit_tools::ToolRegistry;

use crate::config::{ExecutionEnvPolicy, PersistenceConfig, PersistenceType};
use crate::job_file_store::JobFileStore;
use crate::skill_catalog::SkillCatalog;
use crate::task_file_store::TaskFileStore;
use crate::work_file_store::WorkFileStore;

#[derive(Clone)]
pub struct OrbitContext {
    pub(crate) store: Store,
    pub(crate) policy: PolicyEngine,
    pub(crate) registry: Arc<ToolRegistry>,
    pub(crate) task_store: TaskFileStore,
    pub(crate) work_file_store: WorkFileStore,
    pub(crate) job_file_store: JobFileStore,
    pub(crate) skill_catalog: SkillCatalog,
    pub(crate) execution_env_policy: ExecutionEnvPolicy,
    pub(crate) persistence: PersistenceConfig,
    pub(crate) task_approval_required_for_agent: bool,
    pub(crate) work_persistence_type: PersistenceType,
    pub(crate) job_persistence_type: PersistenceType,
}
