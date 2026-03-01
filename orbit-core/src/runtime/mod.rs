pub mod audit;
pub mod event_bus;
pub mod execute;
pub mod mutation;
pub mod pipeline;

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
use orbit_types::{Audit, Job, OrbitError, ResolvedIdentity};
use serde_json::Value;

use crate::OrbitContext;
use crate::config::{PersistenceType, RuntimeConfig};
use crate::identity_catalog::{IdentityCatalog, compile_identity_block};
use crate::skill_catalog::SkillCatalog;

#[derive(Clone)]
pub struct OrbitRuntime {
    pub(crate) context: OrbitContext,
    pub event_bus: event_bus::EventBus,
}

impl OrbitRuntime {
    pub fn initialize() -> Result<Self, OrbitError> {
        let data_root = Self::default_data_root();
        Self::from_data_root(&data_root)
    }

    pub fn from_data_root(data_root: &Path) -> Result<Self, OrbitError> {
        let runtime_config = RuntimeConfig::load_from_data_root(data_root)?;
        let db_path = if runtime_config.persistence.watch.path
            == runtime_config.persistence.audit.path
        {
            runtime_config.persistence.watch.path.clone()
        } else {
            return Err(OrbitError::InvalidInput(
                "watch.persistence.path and audit.persistence.path must match in v2.1".to_string(),
            ));
        };
        let store = Store::open(&db_path)?;

        let task_store = task_store_file(Self::task_root_path(&runtime_config))?;
        let work_store = match runtime_config.persistence.work.persistence_type {
            PersistenceType::File => work_store_file(runtime_config.persistence.work.path.clone())?,
            PersistenceType::Sqlite => work_store_sqlite(Self::sqlite_store_for_entity(
                &store,
                &db_path,
                &runtime_config.persistence.work.path,
            )?),
        };
        let job_store = match runtime_config.persistence.job.persistence_type {
            PersistenceType::File => job_store_file(runtime_config.persistence.job.path.clone())?,
            PersistenceType::Sqlite => job_store_sqlite(Self::sqlite_store_for_entity(
                &store,
                &db_path,
                &runtime_config.persistence.job.path,
            )?),
        };
        let tool_store = tool_store_sqlite(store.clone());
        let watch_store = watch_store_sqlite(store.clone());
        let audit_store = audit_store_sqlite(store.clone());
        let audit_event_store = audit_event_store_sqlite(store.clone());
        let agent_session_store = agent_session_store_sqlite(store.clone());
        let lock_store = lock_store_sqlite(store.clone());

        let skill_root = Self::skill_root_path(&runtime_config);
        let skill_catalog = SkillCatalog::new(skill_root);
        skill_catalog.ensure_layout()?;
        let identity_catalog = IdentityCatalog::new(
            runtime_config.identity.root.clone(),
            runtime_config.identity.role_overrides.clone(),
        );

        let mut registry = ToolRegistry::new();
        registry.register_builtins();
        Self::load_external_tools(&store, &mut registry)?;
        let execution_env_policy = runtime_config.execution_env.clone();
        let persistence = runtime_config.persistence.clone();
        let task_approval_required_for_agent = runtime_config.task_approval.required_for_agent;
        let task_delegate_approval = runtime_config.task_approval.delegate_approval;

        Ok(Self {
            context: OrbitContext {
                store,
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
            },
            event_bus: event_bus::EventBus::default(),
        })
    }

    pub fn in_memory() -> Result<Self, OrbitError> {
        let store = Store::open_in_memory()?;
        let task_store = task_store_file(std::env::temp_dir().join(format!(
            "orbit-task-store-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )))?;
        let work_store = work_store_sqlite(store.clone());
        let job_store = job_store_sqlite(store.clone());
        let tool_store = tool_store_sqlite(store.clone());
        let watch_store = watch_store_sqlite(store.clone());
        let audit_store = audit_store_sqlite(store.clone());
        let audit_event_store = audit_event_store_sqlite(store.clone());
        let agent_session_store = agent_session_store_sqlite(store.clone());
        let lock_store = lock_store_sqlite(store.clone());
        let skill_root = std::env::temp_dir().join(format!(
            "orbit-skill-store-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let skill_catalog = SkillCatalog::new(skill_root);
        skill_catalog.ensure_layout()?;
        let identity_root = std::env::temp_dir().join(format!(
            "orbit-identity-store-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let identity_catalog = IdentityCatalog::new(identity_root, Default::default());
        let mut registry = ToolRegistry::new();
        registry.register_builtins();
        Self::load_external_tools(&store, &mut registry)?;
        let runtime_config = RuntimeConfig::default();
        let task_approval_required_for_agent = runtime_config.task_approval.required_for_agent;
        let task_delegate_approval = runtime_config.task_approval.delegate_approval;

        Ok(Self {
            context: OrbitContext {
                store,
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
                execution_env_policy: runtime_config.execution_env,
                persistence: runtime_config.persistence,
                task_approval_required_for_agent,
                task_delegate_approval,
            },
            event_bus: event_bus::EventBus::default(),
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

    pub fn with_policy(mut self, policy: PolicyEngine) -> Self {
        self.context.policy = policy;
        self
    }

    pub fn list_audits(&self, limit: usize) -> Result<Vec<Audit>, OrbitError> {
        self.context.audit_store.list_audits(limit)
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<Job>, OrbitError> {
        self.context.job_store.get_job(job_id)
    }

    pub fn execution_env_config(&self) -> (bool, Vec<String>) {
        (
            self.context.execution_env_policy.inherit(),
            self.context.execution_env_policy.pass().to_vec(),
        )
    }

    pub fn persistence_config_json(&self) -> Value {
        self.context.persistence.as_json_value()
    }

    pub fn task_approval_required_for_agent(&self) -> bool {
        self.context.task_approval_required_for_agent
    }

    pub fn task_delegate_approval(&self) -> bool {
        self.context.task_delegate_approval
    }

    pub fn identity_root(&self) -> PathBuf {
        self.context.identity_catalog.root().to_path_buf()
    }

    pub fn identity_role_overrides(&self) -> std::collections::BTreeMap<String, String> {
        self.context
            .identity_catalog
            .role_overrides()
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect()
    }

    pub fn resolve_identity(&self, identity_id: &str) -> Result<ResolvedIdentity, OrbitError> {
        self.context.identity_catalog.resolve(identity_id)
    }

    pub fn compile_identity_block(&self, identity: &ResolvedIdentity) -> String {
        compile_identity_block(identity)
    }

    pub fn run_jobs(&self) -> Result<usize, OrbitError> {
        self.run_due_jobs(Utc::now())
    }

    pub fn trigger_watch_once(&self, path: &str) -> Result<(), OrbitError> {
        self.trigger_watch_path(path)
    }

    pub fn default_data_root() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".orbit")
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
}
