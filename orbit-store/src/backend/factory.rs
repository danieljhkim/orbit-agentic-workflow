use std::path::PathBuf;
use std::sync::Arc;

use orbit_types::OrbitError;

use super::contracts::{
    AgentSessionStoreBackend, AuditEventStoreBackend, AuditStoreBackend, JobStoreBackend,
    LockStoreBackend, TaskStoreBackend, ToolStoreBackend, WatchStoreBackend, WorkStoreBackend,
};
use super::sqlite_backends::{
    SqliteAgentSessionStoreBackend, SqliteAuditEventStoreBackend, SqliteAuditStoreBackend,
    SqliteJobStoreBackend, SqliteLockStoreBackend, SqliteTaskStoreBackend, SqliteToolStoreBackend,
    SqliteWatchStoreBackend, SqliteWorkStoreBackend,
};
use crate::Store;
use crate::file::job_store::JobFileStore;
use crate::file::task_store::TaskFileStore;
use crate::file::work_store::WorkFileStore;

pub fn task_store_file(root: PathBuf) -> Result<Arc<dyn TaskStoreBackend>, OrbitError> {
    let store = TaskFileStore::new(root);
    store.ensure_layout()?;
    Ok(Arc::new(store))
}

pub fn task_store_sqlite(store: Store) -> Arc<dyn TaskStoreBackend> {
    Arc::new(SqliteTaskStoreBackend { store })
}

pub fn work_store_file(root: PathBuf) -> Result<Arc<dyn WorkStoreBackend>, OrbitError> {
    let store = WorkFileStore::new(root);
    store.ensure_layout()?;
    Ok(Arc::new(store))
}

pub fn work_store_sqlite(store: Store) -> Arc<dyn WorkStoreBackend> {
    Arc::new(SqliteWorkStoreBackend { store })
}

pub fn job_store_file(root: PathBuf) -> Result<Arc<dyn JobStoreBackend>, OrbitError> {
    let store = JobFileStore::new(root);
    store.ensure_layout()?;
    Ok(Arc::new(store))
}

pub fn job_store_sqlite(store: Store) -> Arc<dyn JobStoreBackend> {
    Arc::new(SqliteJobStoreBackend { store })
}

pub fn tool_store_sqlite(store: Store) -> Arc<dyn ToolStoreBackend> {
    Arc::new(SqliteToolStoreBackend { store })
}

pub fn watch_store_sqlite(store: Store) -> Arc<dyn WatchStoreBackend> {
    Arc::new(SqliteWatchStoreBackend { store })
}

pub fn audit_store_sqlite(store: Store) -> Arc<dyn AuditStoreBackend> {
    Arc::new(SqliteAuditStoreBackend { store })
}

pub fn audit_event_store_sqlite(store: Store) -> Arc<dyn AuditEventStoreBackend> {
    Arc::new(SqliteAuditEventStoreBackend { store })
}

pub fn agent_session_store_sqlite(store: Store) -> Arc<dyn AgentSessionStoreBackend> {
    Arc::new(SqliteAgentSessionStoreBackend { store })
}

pub fn lock_store_sqlite(store: Store) -> Arc<dyn LockStoreBackend> {
    Arc::new(SqliteLockStoreBackend { store })
}
