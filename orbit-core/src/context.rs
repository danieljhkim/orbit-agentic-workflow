use std::path::PathBuf;
use std::sync::Arc;

use orbit_policy::PolicyEngine;
use orbit_store::{
    ActivityStoreBackend, AuditEventStoreBackend, JobStoreBackend, TaskStoreBackend,
    ToolStoreBackend,
};
use orbit_tools::ToolRegistry;

use crate::config::{CodexExecutionPolicy, ExecutionEnvPolicy, PersistenceConfig};
use crate::skill_catalog::SkillCatalog;

const ORBIT_TASK_ACTOR_KIND: &str = "ORBIT_TASK_ACTOR_KIND";
const ORBIT_TASK_ACTOR_LABEL: &str = "ORBIT_TASK_ACTOR_LABEL";
const LEGACY_ORBIT_TASK_ACTOR_IDENTITY_ID: &str = "ORBIT_TASK_ACTOR_IDENTITY_ID";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    Human,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorIdentity {
    pub kind: ActorKind,
    pub label: String,
}

impl ActorIdentity {
    pub fn human(label: impl Into<String>) -> Self {
        Self {
            kind: ActorKind::Human,
            label: normalize_actor_label(label.into(), "human"),
        }
    }

    pub fn agent(label: impl Into<String>) -> Self {
        Self {
            kind: ActorKind::Agent,
            label: normalize_actor_label(label.into(), "agent"),
        }
    }

    pub(crate) fn from_env() -> Self {
        let kind_raw = std::env::var(ORBIT_TASK_ACTOR_KIND).ok();
        let actor_label = std::env::var(ORBIT_TASK_ACTOR_LABEL)
            .ok()
            .or_else(|| std::env::var(LEGACY_ORBIT_TASK_ACTOR_IDENTITY_ID).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        match kind_raw.as_deref() {
            Some("agent") => actor_label
                .map(Self::agent)
                .unwrap_or_else(|| Self::agent("agent")),
            _ if actor_label.is_some() => Self::agent(actor_label.unwrap_or_default()),
            _ => Self::default(),
        }
    }
}

impl Default for ActorIdentity {
    fn default() -> Self {
        Self::human("human")
    }
}

#[derive(Clone)]
pub struct OrbitContext {
    pub(crate) data_root: PathBuf,
    pub(crate) task_store: Arc<dyn TaskStoreBackend>,
    pub(crate) activity_store: Arc<dyn ActivityStoreBackend>,
    pub(crate) job_store: Arc<dyn JobStoreBackend>,
    pub(crate) tool_store: Arc<dyn ToolStoreBackend>,
    pub(crate) audit_event_store: Arc<dyn AuditEventStoreBackend>,
    pub(crate) policy: PolicyEngine,
    pub(crate) registry: Arc<ToolRegistry>,
    pub(crate) skill_catalog: SkillCatalog,
    pub(crate) execution_env_policy: ExecutionEnvPolicy,
    pub(crate) codex_execution_policy: CodexExecutionPolicy,
    pub(crate) persistence: PersistenceConfig,
    pub(crate) user_name: String,
    pub(crate) actor: ActorIdentity,
    pub(crate) task_approval_required_for_agent: bool,
    pub(crate) task_delegate_approval: bool,
}

fn normalize_actor_label(label: String, default_label: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        default_label.to_string()
    } else {
        label.to_string()
    }
}
