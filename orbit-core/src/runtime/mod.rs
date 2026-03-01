pub mod audit;
pub mod builder;
pub mod event_bus;
pub mod execute;
pub mod mutation;
pub mod pipeline;

use std::path::{Path, PathBuf};

use chrono::Utc;
use orbit_policy::PolicyEngine;
use orbit_types::{Audit, Job, OrbitError, ResolvedIdentity};
use serde_json::Value;

use crate::OrbitContext;
use crate::identity_catalog::compile_identity_block;

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
        Ok(Self {
            context: builder::build_context_from_data_root(data_root)?,
            event_bus: event_bus::EventBus::default(),
        })
    }

    pub fn in_memory() -> Result<Self, OrbitError> {
        Ok(Self {
            context: builder::build_context_in_memory()?,
            event_bus: event_bus::EventBus::default(),
        })
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
}
