use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use orbit_policy::PolicyEngine;
use orbit_store::{
    Store, agent_session_store_sqlite, audit_event_store_sqlite, audit_store_sqlite,
    job_store_file, job_store_sqlite, lock_store_sqlite, task_store_file, tool_store_sqlite,
    watch_store_sqlite, work_store_file, work_store_sqlite,
};
use orbit_tools::ToolRegistry;
use orbit_tools::external::ExternalTool;
use orbit_types::OrbitError;

use crate::OrbitContext;
use crate::config::{PersistenceType, RuntimeConfig};
use crate::identity_catalog::IdentityCatalog;
use crate::skill_catalog::SkillCatalog;

pub(crate) fn build_context_from_data_root(data_root: &Path) -> Result<OrbitContext, OrbitError> {
    let runtime_config = RuntimeConfig::load_from_data_root(data_root)?;
    let db_path = if runtime_config.persistence.watch.path == runtime_config.persistence.audit.path
    {
        runtime_config.persistence.watch.path.clone()
    } else {
        return Err(OrbitError::InvalidInput(
            "watch.persistence.path and audit.persistence.path must match in v2.1".to_string(),
        ));
    };
    let store = Store::open(&db_path)?;

    let task_store = task_store_file(task_root_path(&runtime_config))?;
    let work_store = match runtime_config.persistence.work.persistence_type {
        PersistenceType::File => work_store_file(runtime_config.persistence.work.path.clone())?,
        PersistenceType::Sqlite => work_store_sqlite(sqlite_store_for_entity(
            &store,
            &db_path,
            &runtime_config.persistence.work.path,
        )?),
    };
    let job_store = match runtime_config.persistence.job.persistence_type {
        PersistenceType::File => job_store_file(runtime_config.persistence.job.path.clone())?,
        PersistenceType::Sqlite => job_store_sqlite(sqlite_store_for_entity(
            &store,
            &db_path,
            &runtime_config.persistence.job.path,
        )?),
    };

    build_context_common(
        store,
        data_root.to_path_buf(),
        runtime_config,
        task_store,
        work_store,
        job_store,
    )
}

pub(crate) fn build_context_in_memory() -> Result<OrbitContext, OrbitError> {
    let store = Store::open_in_memory()?;
    let task_store = task_store_file(std::env::temp_dir().join(format!(
        "orbit-task-store-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )))?;
    let work_store = work_store_sqlite(store.clone());
    let job_store = job_store_sqlite(store.clone());
    let runtime_config = RuntimeConfig::default();
    let data_root = runtime_config
        .persistence
        .task
        .path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    build_context_common(
        store,
        data_root,
        runtime_config,
        task_store,
        work_store,
        job_store,
    )
}

fn build_context_common(
    store: Store,
    data_root: PathBuf,
    runtime_config: RuntimeConfig,
    task_store: Arc<dyn orbit_store::TaskStoreBackend>,
    work_store: Arc<dyn orbit_store::WorkStoreBackend>,
    job_store: Arc<dyn orbit_store::JobStoreBackend>,
) -> Result<OrbitContext, OrbitError> {
    let tool_store = tool_store_sqlite(store.clone());
    let watch_store = watch_store_sqlite(store.clone());
    let audit_store = audit_store_sqlite(store.clone());
    let audit_event_store = audit_event_store_sqlite(store.clone());
    let agent_session_store = agent_session_store_sqlite(store.clone());
    let lock_store = lock_store_sqlite(store.clone());

    let skill_root = skill_root_path(&runtime_config);
    let skill_catalog = SkillCatalog::new(skill_root);
    skill_catalog.ensure_layout()?;

    let identity_catalog = IdentityCatalog::new(
        runtime_config.identity.root.clone(),
        runtime_config.identity.role_overrides.clone(),
    );

    let mut registry = ToolRegistry::new();
    registry.register_builtins();
    load_external_tools(&store, &mut registry)?;

    let execution_env_policy = runtime_config.execution_env.clone();
    let persistence = runtime_config.persistence.clone();
    let task_approval_required_for_agent = runtime_config.task_approval.required_for_agent;
    let task_delegate_approval = runtime_config.task_approval.delegate_approval;

    Ok(OrbitContext {
        data_root,
        task_store,
        work_store,
        job_store,
        tool_store,
        watch_store,
        audit_store,
        audit_event_store,
        agent_session_store,
        lock_store,
        policy: PolicyEngine::new_local_default_allow(),
        registry: Arc::new(registry),
        skill_catalog,
        identity_catalog,
        execution_env_policy,
        persistence,
        task_approval_required_for_agent,
        task_delegate_approval,
    })
}

fn sqlite_store_for_entity(
    default_store: &Store,
    default_path: &Path,
    entity_path: &Path,
) -> Result<Store, OrbitError> {
    if entity_path == default_path {
        return Ok(default_store.clone());
    }
    Store::open(entity_path)
}

fn load_external_tools(store: &Store, registry: &mut ToolRegistry) -> Result<(), OrbitError> {
    let stored_tools = store.list_tools()?;
    for tool in stored_tools {
        if !tool.builtin && tool.enabled && !registry.has(&tool.name) {
            registry.register(ExternalTool {
                name: tool.name,
                path: tool.path,
                description: tool.description,
            });
        }
    }
    Ok(())
}

fn task_root_path(runtime_config: &RuntimeConfig) -> PathBuf {
    if let Ok(value) = std::env::var("ORBIT_TASK_ROOT")
        && !value.trim().is_empty()
    {
        return PathBuf::from(value);
    }
    runtime_config.persistence.task.path.clone()
}

fn skill_root_path(runtime_config: &RuntimeConfig) -> PathBuf {
    if let Ok(value) = std::env::var("ORBIT_SKILL_ROOT")
        && !value.trim().is_empty()
    {
        return PathBuf::from(value);
    }
    runtime_config.persistence.skill.path.clone()
}
