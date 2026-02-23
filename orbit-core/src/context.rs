use std::sync::Arc;

use orbit_policy::PolicyEngine;
use orbit_store::Store;
use orbit_tools::ToolRegistry;

use crate::task_file_store::TaskFileStore;

#[derive(Clone)]
pub struct OrbitContext {
    pub(crate) store: Store,
    pub(crate) policy: PolicyEngine,
    pub(crate) registry: Arc<ToolRegistry>,
    pub(crate) task_store: TaskFileStore,
}
