use orbit_common::types::{
    OrbitError, OrbitEvent, Task, normalize_task_dependencies, prune_missing_context_files,
    validate_task_dependencies,
};
use orbit_store::{TaskCreateParams as StoreTaskCreateParams, friction_bounty};

use crate::OrbitRuntime;
use crate::runtime::TaskRecordUpdateParams;

use super::helpers::{authored_role_value, build_task_comments, effective_actor_label};
use super::params::TaskAddParams;
use super::paths::{
    context_files_pruned_history_entry, context_workspace_root,
    emit_graph_unavailable_warning_if_needed, normalize_context_files_for_write,
    normalize_workspace_path,
};

impl OrbitRuntime {
    pub fn add_task(&self, params: TaskAddParams) -> Result<Task, OrbitError> {
        self.add_task_with_identity(params, None, None)
    }

    pub fn add_task_with_identity(
        &self,
        params: TaskAddParams,
        agent: Option<String>,
        model: Option<String>,
    ) -> Result<Task, OrbitError> {
        let (canonical_agent, canonical_model) =
            self.canonical_agent_model_identity(agent.as_deref(), model.as_deref());
        let actor = self.actor().clone();
        let effective_label = effective_actor_label(
            &actor.label,
            canonical_agent.as_deref(),
            canonical_model.as_deref(),
        );
        let initial_status = orbit_common::types::TaskStatus::Proposed;
        let uses_system_identity = params.system_created;
        let create_label = if uses_system_identity {
            "system".to_string()
        } else {
            effective_label.clone()
        };
        let planned_by = authored_role_value(params.plan.as_str(), &create_label);
        let comments = build_task_comments(params.comment.clone(), create_label.as_str())?;
        let workspace_path =
            normalize_workspace_path(&self.paths().repo_root, params.workspace_path.as_deref())?;
        let dependencies = normalize_task_dependencies(params.dependencies.clone())?;
        validate_task_dependencies(&self.list_tasks()?, None, &dependencies)?;

        let prune_root = context_workspace_root(&self.paths().repo_root, workspace_path.as_deref());
        let normalized_context_files =
            normalize_context_files_for_write(params.context_files.clone(), &prune_root)?;
        emit_graph_unavailable_warning_if_needed(&normalized_context_files, self.data_root_path());
        let (kept_context_files, dropped_context_files) =
            prune_missing_context_files(&prune_root, normalized_context_files);

        let task = self.with_mutation(|| {
            let task = self.stores().tasks().create(StoreTaskCreateParams {
                actor: create_label.clone(),
                parent_id: params.parent_id.clone(),
                title: params.title.clone(),
                description: params.description.clone(),
                acceptance_criteria: params.acceptance_criteria.clone(),
                dependencies: dependencies.clone(),
                plan: params.plan.clone(),
                execution_summary: String::new(),
                context_files: kept_context_files.clone(),
                workspace_path: workspace_path.clone(),
                repo_root: None,
                created_by: Some(create_label.clone()),
                planned_by,
                implemented_by: None,
                agent: canonical_agent.clone(),
                model: canonical_model.clone(),
                status: initial_status,
                priority: params.priority,
                complexity: params.complexity,
                task_type: params.task_type,
                pr_number: None,
                source_task_id: params.source_task_id.clone(),
                comments: comments.clone(),
            })?;
            Ok((
                task.clone(),
                OrbitEvent::TaskAdded {
                    id: task.id.clone(),
                },
            ))
        })?;

        if self.scoring_enabled()
            && params.task_type.counts_toward_friction_bounty()
            && let Some(model) = &canonical_model
        {
            emit_friction_reported_trace(
                &task.id,
                canonical_agent
                    .as_deref()
                    .unwrap_or(effective_label.as_str()),
                model,
                &task.title,
            );
            let _ = friction_bounty::record_friction_reported(&self.paths().scoreboard_dir, model);
        }

        let task = if dropped_context_files.is_empty() {
            task
        } else {
            self.stores().tasks().update(
                &task.id,
                TaskRecordUpdateParams {
                    actor: create_label.clone(),
                    append_history: vec![context_files_pruned_history_entry(
                        &create_label,
                        &dropped_context_files,
                    )],
                    ..Default::default()
                },
            )?
        };

        Ok(task)
    }
}

fn emit_friction_reported_trace(task_id: &str, agent: &str, model: &str, summary: &str) {
    tracing::warn!(
        target: "orbit.friction.reported",
        task_id,
        agent,
        model,
        summary,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use orbit_common::types::{TaskStatus, TaskType};
    use tempfile::tempdir;
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    fn test_runtime() -> (tempfile::TempDir, OrbitRuntime) {
        let root = tempdir().expect("create tempdir");
        let global_root = root.path().join("global");
        let repo_root = root.path().join("repo");
        let workspace_root = repo_root.join(".orbit");
        std::fs::create_dir_all(&global_root).expect("create global root");
        std::fs::create_dir_all(&workspace_root).expect("create workspace root");
        let runtime =
            OrbitRuntime::from_roots(&global_root, &workspace_root).expect("build test runtime");
        (root, runtime)
    }

    #[test]
    fn task_add_enters_proposed_and_requires_approval_before_backlog() {
        let (_root, runtime) = test_runtime();

        let task = runtime
            .add_task(TaskAddParams {
                title: "Create orbit hello".to_string(),
                description: "Add a small hello file.".to_string(),
                acceptance_criteria: vec!["orbit-hello.txt exists.".to_string()],
                workspace_path: Some(".".to_string()),
                ..Default::default()
            })
            .expect("human task add succeeds");

        assert_eq!(task.status, TaskStatus::Proposed);

        let approved = runtime
            .approve_task(&task.id, Some("LGTM".to_string()), None)
            .expect("proposed task can be approved into backlog");
        assert_eq!(approved.status, TaskStatus::Backlog);

        let started = runtime
            .start_task(&task.id, Some("start approved task".to_string()), None)
            .expect("backlog task starts directly");
        assert_eq!(started.status, TaskStatus::InProgress);
    }

    #[test]
    fn friction_task_submission_emits_one_tracing_event() {
        let (_root, runtime) = test_runtime();
        let subscriber = RecordingSubscriber::default();
        let recorder = subscriber.clone();
        let dispatch = tracing::Dispatch::new(subscriber);

        let task = tracing::dispatcher::with_default(&dispatch, || {
            runtime
                .add_task_with_identity(
                    TaskAddParams {
                        title: "Friction reported on ORB-1011".to_string(),
                        description: "Tooling got stuck.".to_string(),
                        acceptance_criteria: vec!["Report is visible.".to_string()],
                        task_type: TaskType::Friction,
                        workspace_path: Some(".".to_string()),
                        ..Default::default()
                    },
                    Some("codex".to_string()),
                    Some("gpt-5.5".to_string()),
                )
                .expect("friction task add succeeds")
        });

        let events = recorder.events_for_target("orbit.friction.reported");
        assert_eq!(events.len(), 1, "expected exactly one friction event");
        let fields = &events[0].fields;
        assert_eq!(fields.get("task_id"), Some(&task.id));
        assert_eq!(fields.get("agent"), Some(&"codex".to_string()));
        assert_eq!(fields.get("model"), Some(&"gpt-5.5".to_string()));
        assert_eq!(
            fields.get("summary"),
            Some(&"Friction reported on ORB-1011".to_string())
        );
    }

    #[derive(Clone, Default)]
    struct RecordingSubscriber {
        events: Arc<Mutex<Vec<RecordedEvent>>>,
    }

    impl RecordingSubscriber {
        fn events_for_target(&self, target: &str) -> Vec<RecordedEvent> {
            self.events
                .lock()
                .expect("events lock")
                .iter()
                .filter(|event| event.target == target)
                .cloned()
                .collect()
        }
    }

    impl Subscriber for RecordingSubscriber {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }

        fn record(&self, _span: &Id, _values: &Record<'_>) {}

        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

        fn event(&self, event: &Event<'_>) {
            let mut visitor = FieldRecorder::default();
            event.record(&mut visitor);
            self.events
                .lock()
                .expect("events lock")
                .push(RecordedEvent {
                    target: event.metadata().target().to_string(),
                    fields: visitor.fields,
                });
        }

        fn enter(&self, _span: &Id) {}

        fn exit(&self, _span: &Id) {}
    }

    #[derive(Clone, Debug)]
    struct RecordedEvent {
        target: String,
        fields: BTreeMap<String, String>,
    }

    #[derive(Default)]
    struct FieldRecorder {
        fields: BTreeMap<String, String>,
    }

    impl Visit for FieldRecorder {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.fields
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }
}
