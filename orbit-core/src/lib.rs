mod job;
pub mod watch;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::Utc;
use orbit_policy::{PolicyContext, PolicyEngine};
use orbit_store::{Store, StoreTx};
use orbit_tools::{ToolContext, ToolRegistry};
use orbit_types::{Audit, OrbitError, OrbitEvent, PolicyDecision, Task};
use serde_json::Value;

#[derive(Clone)]
pub struct OrbitContext {
    pub store: Store,
    pub policy: PolicyEngine,
    pub registry: Arc<ToolRegistry>,
}

#[derive(Clone, Default)]
pub struct EventBus {
    events: Arc<Mutex<Vec<OrbitEvent>>>,
}

impl EventBus {
    pub fn publish(&self, event: OrbitEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    pub fn snapshot(&self) -> Vec<OrbitEvent> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone)]
pub struct OrbitRuntime {
    pub context: OrbitContext,
    pub event_bus: EventBus,
}

impl OrbitRuntime {
    pub fn from_data_root(data_root: &Path) -> Result<Self, OrbitError> {
        let db_path = data_root.join("orbit.db");
        let store = Store::open(&db_path)?;

        let mut registry = ToolRegistry::new();
        registry.register_builtins();

        Ok(Self {
            context: OrbitContext {
                store,
                policy: PolicyEngine::new_local_default_allow(),
                registry: Arc::new(registry),
            },
            event_bus: EventBus::default(),
        })
    }

    pub fn in_memory() -> Result<Self, OrbitError> {
        let store = Store::open_in_memory()?;
        let mut registry = ToolRegistry::new();
        registry.register_builtins();

        Ok(Self {
            context: OrbitContext {
                store,
                policy: PolicyEngine::new_local_default_allow(),
                registry: Arc::new(registry),
            },
            event_bus: EventBus::default(),
        })
    }

    pub fn with_policy(mut self, policy: PolicyEngine) -> Self {
        self.context.policy = policy;
        self
    }

    pub fn run_tool(&self, name: &str, input: Value) -> Result<Value, OrbitError> {
        let decision = self.context.policy.evaluate(&PolicyContext {
            entrypoint: "cli".to_string(),
            tool_name: Some(name.to_string()),
        });

        match decision {
            PolicyDecision::Deny { reason } => {
                self.with_mutation(
                    OrbitEvent::PolicyDenied {
                        tool: name.to_string(),
                    },
                    "policy denied tool execution",
                    |_| Ok(()),
                )?;
                Err(OrbitError::PolicyDenied(reason))
            }
            PolicyDecision::Allow => {
                let output = self
                    .context
                    .registry
                    .execute(name, &ToolContext::default(), input)?;

                self.with_mutation(
                    OrbitEvent::ToolExecuted {
                        name: name.to_string(),
                    },
                    "tool execution completed",
                    |_| Ok(()),
                )?;

                Ok(output)
            }
        }
    }

    pub fn add_task(&self, title: &str) -> Result<Task, OrbitError> {
        self.with_mutation(
            OrbitEvent::TaskAdded {
                id: "pending".to_string(),
            },
            "task created",
            |tx| tx.insert_task(title),
        )
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        self.context.store.list_tasks()
    }

    pub fn list_audits(&self, limit: usize) -> Result<Vec<Audit>, OrbitError> {
        self.context.store.list_audits(limit)
    }

    pub fn run_jobs(&self) -> Result<usize, OrbitError> {
        self.run_due_jobs(Utc::now())
    }

    pub fn run_watch_once(&self, path: &str) -> Result<(), OrbitError> {
        self.trigger_watch_path(path)
    }

    fn with_mutation<T, F>(&self, event: OrbitEvent, message: &str, op: F) -> Result<T, OrbitError>
    where
        F: FnOnce(&mut StoreTx<'_>) -> Result<T, OrbitError>,
    {
        let result = self.context.store.with_mutation(&event, message, op)?;
        self.event_bus.publish(event);
        Ok(result)
    }

    pub fn default_data_root() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".orbit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_policy::PolicyEngine;
    use orbit_types::{JobStatus, OrbitEvent};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn policy_denied_records_audit_and_no_side_effects() {
        let runtime = OrbitRuntime::in_memory()
            .expect("runtime")
            .with_policy(PolicyEngine::new_local_default_allow().deny_tool("fs.read"));

        let result = runtime.run_tool("fs.read", json!({"path": "missing"}));
        assert!(matches!(result, Err(OrbitError::PolicyDenied(_))));

        let audits = runtime.list_audits(10).expect("audits");
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].event_type, "PolicyDenied");
    }

    #[test]
    fn successful_tool_execution_persists_audit_and_event() {
        let dir = tempdir().expect("temp dir");
        let file = dir.path().join("sample.txt");
        std::fs::write(&file, "ok").expect("write file");

        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let output = runtime
            .run_tool("fs.read", json!({"path": file.to_string_lossy()}))
            .expect("tool succeeds");

        assert_eq!(output["content"], "ok");

        let audits = runtime.list_audits(10).expect("audits");
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].event_type, "ToolExecuted");

        let events = runtime.event_bus.snapshot();
        assert!(matches!(
            events.first(),
            Some(OrbitEvent::ToolExecuted { name }) if name == "fs.read"
        ));
    }

    #[test]
    fn mutation_boundary_always_emits_audit() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let _ = runtime.add_task("ship orbit").expect("add task");

        let tasks = runtime.list_tasks().expect("tasks");
        let audits = runtime.list_audits(10).expect("audits");

        assert_eq!(tasks.len(), 1);
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0].event_type, "TaskAdded");
    }

    #[test]
    fn job_run_does_not_double_execute_due_job() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let now = Utc::now();
        let job = runtime
            .context
            .store
            .insert_job("demo", "noop", now)
            .expect("insert job");

        let first = runtime.run_due_jobs(now).expect("first run");
        let second = runtime.run_due_jobs(now).expect("second run");

        assert_eq!(first, 1);
        assert_eq!(second, 0);

        let status = runtime
            .context
            .store
            .get_job_status(&job.id)
            .expect("status")
            .expect("present");
        assert_eq!(status, JobStatus::Complete);
    }

    #[test]
    fn job_run_skips_when_global_lock_held() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        assert!(
            runtime
                .context
                .store
                .try_lock(orbit_store::Store::global_job_lock_name())
                .expect("lock")
        );

        let ran = runtime.run_jobs().expect("run jobs");
        assert_eq!(ran, 0);

        let _ = runtime
            .context
            .store
            .unlock(orbit_store::Store::global_job_lock_name());
    }

    #[test]
    fn watch_debounce_coalesces_burst_events() {
        let mut d = watch::DebounceQueueOne::new(100);
        let first = d.on_event("a.txt".to_string(), 0);
        let second = d.on_event("b.txt".to_string(), 10);
        let third = d.on_event("c.txt".to_string(), 20);

        assert_eq!(first.as_deref(), Some("a.txt"));
        assert!(second.is_none());
        assert!(third.is_none());

        assert!(d.on_tick(50).is_none());
        assert_eq!(d.on_tick(100).as_deref(), Some("c.txt"));
    }
}
