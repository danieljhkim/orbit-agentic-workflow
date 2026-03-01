pub mod audit;
pub mod event_bus;
pub mod execute;
pub mod mutation;
pub mod pipeline;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use orbit_policy::PolicyEngine;
use orbit_tools::ToolRegistry;
use orbit_tools::external::ExternalTool;
use orbit_types::{Audit, Job, OrbitError, ResolvedIdentity};
use serde_json::Value;

use crate::OrbitContext;
use crate::config::{PersistenceType, RuntimeConfig};
use crate::identity_catalog::{IdentityCatalog, compile_identity_block};
use crate::job_file_store::JobFileStore;
use crate::skill_catalog::SkillCatalog;
use crate::task_file_store::TaskFileStore;
use crate::work_file_store::WorkFileStore;

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
        let store = orbit_store::Store::open(&db_path)?;

        if runtime_config.persistence.work.persistence_type == PersistenceType::Sqlite
            && runtime_config.persistence.work.path != db_path
        {
            return Err(OrbitError::InvalidInput(
                "work.persistence.path must match watch/audit sqlite path in v2.1".to_string(),
            ));
        }
        if runtime_config.persistence.job.persistence_type == PersistenceType::Sqlite
            && runtime_config.persistence.job.path != db_path
        {
            return Err(OrbitError::InvalidInput(
                "job.persistence.path must match watch/audit sqlite path in v2.1".to_string(),
            ));
        }

        let work_file_root =
            if runtime_config.persistence.work.persistence_type == PersistenceType::File {
                runtime_config.persistence.work.path.clone()
            } else {
                data_root.join("works")
            };
        let work_file_store = WorkFileStore::new(work_file_root);
        if runtime_config.persistence.work.persistence_type == PersistenceType::File {
            work_file_store.ensure_layout()?;
            let legacy_works = store.list_works(true)?;
            let _ = work_file_store.migrate_from_sqlite_works(&legacy_works)?;
        }

        let job_file_root =
            if runtime_config.persistence.job.persistence_type == PersistenceType::File {
                runtime_config.persistence.job.path.clone()
            } else {
                data_root.join("jobs")
            };
        let job_file_store = JobFileStore::new(job_file_root);
        if runtime_config.persistence.job.persistence_type == PersistenceType::File {
            job_file_store.ensure_layout()?;
            let legacy_jobs = store.list_jobs(true)?;
            let mut runs_by_job = Vec::with_capacity(legacy_jobs.len());
            for job in &legacy_jobs {
                runs_by_job.push((job.job_id.clone(), store.list_job_runs(&job.job_id)?));
            }
            let _ = job_file_store.migrate_from_sqlite(&legacy_jobs, &runs_by_job)?;
        }

        let task_root = Self::task_root_path(&runtime_config);
        let task_store = TaskFileStore::new(task_root);
        task_store.ensure_layout()?;
        let legacy_tasks = store.list_tasks()?;
        let _ = task_store.migrate_from_sqlite_tasks(&legacy_tasks)?;
        let skill_root = Self::skill_root_path(&runtime_config);
        let skill_catalog = SkillCatalog::new(skill_root);
        skill_catalog.ensure_layout()?;
        Self::export_legacy_skills_to_files(&store, &skill_catalog)?;
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
        let work_persistence_type = persistence.work.persistence_type;
        let job_persistence_type = persistence.job.persistence_type;

        Ok(Self {
            context: OrbitContext {
                store,
                policy: PolicyEngine::new_local_default_allow(),
                registry: Arc::new(registry),
                task_store,
                work_file_store,
                job_file_store,
                skill_catalog,
                identity_catalog,
                execution_env_policy,
                persistence,
                task_approval_required_for_agent,
                task_delegate_approval,
                work_persistence_type,
                job_persistence_type,
            },
            event_bus: event_bus::EventBus::default(),
        })
    }

    pub fn in_memory() -> Result<Self, OrbitError> {
        let store = orbit_store::Store::open_in_memory()?;
        let task_root = std::env::temp_dir().join(format!(
            "orbit-task-store-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let task_store = TaskFileStore::new(task_root);
        task_store.ensure_layout()?;
        let work_root = std::env::temp_dir().join(format!(
            "orbit-work-store-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let work_file_store = WorkFileStore::new(work_root);
        work_file_store.ensure_layout()?;
        let job_root = std::env::temp_dir().join(format!(
            "orbit-job-store-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let job_file_store = JobFileStore::new(job_root);
        job_file_store.ensure_layout()?;
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
                policy: PolicyEngine::new_local_default_allow(),
                registry: Arc::new(registry),
                task_store,
                work_file_store,
                job_file_store,
                skill_catalog,
                identity_catalog,
                execution_env_policy: runtime_config.execution_env,
                persistence: runtime_config.persistence,
                task_approval_required_for_agent,
                task_delegate_approval,
                work_persistence_type: PersistenceType::Sqlite,
                job_persistence_type: PersistenceType::Sqlite,
            },
            event_bus: event_bus::EventBus::default(),
        })
    }

    fn load_external_tools(
        store: &orbit_store::Store,
        registry: &mut ToolRegistry,
    ) -> Result<(), OrbitError> {
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
        self.context.store.list_audits(limit)
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<Job>, OrbitError> {
        if self.context.job_persistence_type == PersistenceType::File {
            self.context.job_file_store.get_job(job_id)
        } else {
            self.context.store.get_job(job_id)
        }
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

    fn export_legacy_skills_to_files(
        store: &orbit_store::Store,
        skill_catalog: &SkillCatalog,
    ) -> Result<(), OrbitError> {
        let legacy_skills = store.list_skills()?;
        for skill in legacy_skills {
            let skill_dir = skill_catalog.root().join(&skill.name);
            if skill_dir.join("SKILL.md").exists() {
                continue;
            }
            std::fs::create_dir_all(&skill_dir).map_err(|e| OrbitError::Io(e.to_string()))?;
            let purpose = skill
                .description
                .clone()
                .unwrap_or_else(|| format!("Migrated legacy skill '{}'", skill.name));
            let description = purpose
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let description = description.replace('\\', "\\\\").replace('"', "\\\"");
            let description = format!("\"{description}\"");
            let content = format!(
                "---\nname: {id}\ndescription: {description}\n---\n\n# {id}\n\n## Purpose\n{purpose}\n\n## Behavioral Constraints\n{instructions}\n\n## Output Requirements\n- Return structured output that matches the execution contract.\n",
                id = skill.name,
                description = description,
                instructions = skill.instructions.trim(),
            );
            std::fs::write(skill_dir.join("SKILL.md"), content)
                .map_err(|e| OrbitError::Io(e.to_string()))?;
        }
        Ok(())
    }
}
