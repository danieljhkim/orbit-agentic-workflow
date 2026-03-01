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
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let data_root = resolve_initialize_data_root(&cwd, &Self::default_data_root());
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
        home_dir()
            .map(|home| home.join(".orbit"))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(".orbit")
            })
    }
}

fn resolve_initialize_data_root(cwd: &Path, default_root: &Path) -> PathBuf {
    if let Ok(explicit) = std::env::var("ORBIT_DATA_ROOT")
        && !explicit.trim().is_empty()
    {
        return PathBuf::from(explicit);
    }
    let local_root = cwd.join(".orbit");
    if local_root.join("config.toml").exists() {
        return local_root;
    }
    default_root.to_path_buf()
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Some(PathBuf::from(home));
    }
    if let Ok(profile) = std::env::var("USERPROFILE")
        && !profile.trim().is_empty()
    {
        return Some(PathBuf::from(profile));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::resolve_initialize_data_root;

    #[test]
    fn local_config_has_precedence_over_default_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cwd = dir.path();
        let local_root = cwd.join(".orbit");
        std::fs::create_dir_all(&local_root).expect("create local root");
        std::fs::write(local_root.join("config.toml"), "[task]\n").expect("write config");

        let default_root = dir.path().join("home").join(".orbit");
        let chosen = resolve_initialize_data_root(cwd, &default_root);
        assert_eq!(chosen, local_root);
    }

    #[test]
    fn default_root_used_when_local_config_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cwd = dir.path();
        let default_root = dir.path().join("home").join(".orbit");
        let chosen = resolve_initialize_data_root(cwd, &default_root);
        assert_eq!(chosen, default_root);
    }

    #[test]
    fn orbit_data_root_env_overrides_all() {
        let dir = tempfile::tempdir().expect("tempdir");
        let explicit = dir.path().join("explicit-data");
        let cwd = dir.path();

        // Create a local config that would normally take precedence
        let local_root = cwd.join(".orbit");
        std::fs::create_dir_all(&local_root).expect("create local root");
        std::fs::write(local_root.join("config.toml"), "[task]\n").expect("write config");

        let default_root = dir.path().join("home").join(".orbit");

        // SAFETY: test runs in isolation; no other thread reads this var concurrently.
        unsafe { std::env::set_var("ORBIT_DATA_ROOT", &explicit) };
        let chosen = resolve_initialize_data_root(cwd, &default_root);
        unsafe { std::env::remove_var("ORBIT_DATA_ROOT") };

        assert_eq!(chosen, explicit);
    }
}
