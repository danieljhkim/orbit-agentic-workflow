use std::path::Path;

use orbit_common::types::{Activity, ExecutorType, JobRunState, OrbitError, OrbitEvent};
use orbit_common::utility::fs::write_text_with_parent;
use orbit_store::ActivityCreateParams as StoreWorkCreateParams;
use orbit_store::ActivityUpdateParams as StoreActivityUpdateParams;
use serde_json::Value;

use crate::OrbitRuntime;

/// Shippable default activity assets, seeded under
/// `<orbit_root>/resources/activities/<name>.yaml` on `orbit init`. Keep this
/// list in sync with the workflow YAMLs under `crates/orbit-core/assets/jobs/`:
/// every `target: activity:<name>` reference in a shipped workflow must
/// resolve to an entry here. Reference/example activities (anything under
/// `assets/activities/examples/`) are deliberately excluded — they're
/// fixtures for `crates/orbit-engine/examples/v2_job_runtime_smoke.rs`, not
/// runtime defaults.
pub(crate) const DEFAULT_ACTIVITY_FILES: &[(&str, &str)] = &[
    (
        "agent_implement",
        include_str!("../../assets/activities/agent_implement.yaml"),
    ),
    (
        "dispatch_agent",
        include_str!("../../assets/activities/dispatch_agent.yaml"),
    ),
    (
        "epic_orchestrator",
        include_str!("../../assets/activities/epic_orchestrator.yaml"),
    ),
    (
        "gate_starvation_fail",
        include_str!("../../assets/activities/gate_starvation_fail.yaml"),
    ),
    (
        "git_merge",
        include_str!("../../assets/activities/git_merge.yaml"),
    ),
    (
        "git_push",
        include_str!("../../assets/activities/git_push.yaml"),
    ),
    (
        "invoke_and_wait",
        include_str!("../../assets/activities/invoke_and_wait.yaml"),
    ),
    (
        "list_backlog_tasks",
        include_str!("../../assets/activities/list_backlog_tasks.yaml"),
    ),
    (
        "load_epic",
        include_str!("../../assets/activities/load_epic.yaml"),
    ),
    (
        "pr_open",
        include_str!("../../assets/activities/pr_open.yaml"),
    ),
    (
        "reserve_locks",
        include_str!("../../assets/activities/reserve_locks.yaml"),
    ),
    ("sleep", include_str!("../../assets/activities/sleep.yaml")),
    (
        "summarize_epic",
        include_str!("../../assets/activities/summarize_epic.yaml"),
    ),
    (
        "update_task",
        include_str!("../../assets/activities/update_task.yaml"),
    ),
    (
        "validate_bundles",
        include_str!("../../assets/activities/validate_bundles.yaml"),
    ),
    (
        "worktree_setup",
        include_str!("../../assets/activities/worktree_setup.yaml"),
    ),
];

#[derive(Debug, Clone)]
pub struct ActivityAddParams {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub spec_config: Value,
    pub executor: Option<String>,
    pub workspace_path: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ActivityUpdateParams {
    pub description: Option<String>,
    pub input_schema_json: Option<Value>,
    pub output_schema_json: Option<Value>,
    pub spec_config: Option<Value>,
    pub executor: Option<Option<String>>,
    pub workspace_path: Option<Option<String>>,
    pub created_by: Option<Option<String>>,
    pub is_active: Option<bool>,
}

impl From<ActivityAddParams> for StoreWorkCreateParams {
    fn from(p: ActivityAddParams) -> Self {
        Self {
            id: p.id,
            spec_type: p.spec_type,
            description: p.description,
            input_schema_json: p.input_schema_json,
            output_schema_json: p.output_schema_json,
            spec_config: p.spec_config,
            executor: p.executor,
            workspace_path: p.workspace_path,
            created_by: p.created_by,
        }
    }
}

impl From<ActivityUpdateParams> for StoreActivityUpdateParams {
    fn from(p: ActivityUpdateParams) -> Self {
        Self {
            description: p.description,
            input_schema_json: p.input_schema_json,
            output_schema_json: p.output_schema_json,
            spec_config: p.spec_config,
            executor: p.executor,
            workspace_path: p.workspace_path,
            created_by: p.created_by,
            is_active: p.is_active,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityRunParams {
    pub activity_id: String,
    pub agent_cli: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ActivityRunResult {
    pub activity_id: String,
    pub state: JobRunState,
    pub duration_ms: Option<u64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl OrbitRuntime {
    pub(crate) fn validate_activity_target_exists(
        &self,
        target_type: orbit_common::types::JobTargetType,
        target_id: &str,
    ) -> Result<Activity, OrbitError> {
        let _ = target_type;
        let activity = self.show_activity(target_id)?;
        let skill_refs = activity_skill_refs_from_spec_config(&activity.spec_config)?;
        let _ = self.resolve_activity_skill_refs(&skill_refs)?;
        Ok(activity)
    }

    pub fn add_activity(&self, params: ActivityAddParams) -> Result<Activity, OrbitError> {
        validate_activity_params(&params)?;
        let skill_refs = activity_skill_refs_from_spec_config(&params.spec_config)?;
        let _ = self.resolve_activity_skill_refs(&skill_refs)?;
        let activity = self.stores().activities().add(params.into())?;
        self.record_event(OrbitEvent::ActivityAdded {
            id: activity.id.clone(),
        })?;
        Ok(activity)
    }

    pub fn list_activities(&self, include_inactive: bool) -> Result<Vec<Activity>, OrbitError> {
        self.stores().activities().list(include_inactive)
    }

    pub fn show_activity(&self, id: &str) -> Result<Activity, OrbitError> {
        self.stores()
            .activities()
            .get(id)?
            .ok_or_else(|| OrbitError::ActivityNotFound(id.to_string()))
    }

    pub fn update_activity(
        &self,
        id: &str,
        params: ActivityUpdateParams,
    ) -> Result<Activity, OrbitError> {
        if let Some(spec_config) = params.spec_config.as_ref() {
            ensure_spec_config_object(spec_config)?;
            let skill_refs = activity_skill_refs_from_spec_config(spec_config)?;
            let _ = self.resolve_activity_skill_refs(&skill_refs)?;
        }

        let activity = self.stores().activities().update(id, params.into())?;
        self.record_event(OrbitEvent::ActivityUpdated {
            id: activity.id.clone(),
        })?;
        Ok(activity)
    }

    pub fn delete_activity(&self, id: &str) -> Result<(), OrbitError> {
        let changed = self.stores().activities().disable(id)?;
        if !changed {
            return Err(OrbitError::ActivityNotFound(id.to_string()));
        }
        self.record_event(OrbitEvent::ActivityDisabled { id: id.to_string() })
    }

    pub fn run_activity_now(
        &self,
        params: ActivityRunParams,
    ) -> Result<ActivityRunResult, OrbitError> {
        if params.activity_id.trim().is_empty() {
            return Err(OrbitError::InvalidInput(
                "activity id must not be empty".to_string(),
            ));
        }

        let activity = self.show_activity(&params.activity_id)?;
        validate_activity_executor_for_run(self, &activity)?;
        if activity_requires_agent_cli(&activity.spec_type)
            && params.agent_cli.trim().is_empty()
            && !activity_has_executor(&activity)
        {
            return Err(OrbitError::InvalidInput(
                "agent_cli must not be empty for agent_invoke activities without an explicit executor"
                    .to_string(),
            ));
        }
        self.record_event(OrbitEvent::ActivityRunStarted {
            id: activity.id.clone(),
        })?;
        let outcome = orbit_engine::run_activity_direct(
            self,
            &activity,
            &params.agent_cli,
            params.timeout_seconds,
            false,
        )?;
        if outcome.protocol_violation {
            self.record_event(OrbitEvent::ActivityProtocolViolation {
                id: activity.id.clone(),
                message: outcome
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "agent protocol violation".to_string()),
            })?;
        }
        self.record_event(OrbitEvent::ActivityRunCompleted {
            id: activity.id.clone(),
            state: outcome.state.to_string(),
        })?;

        Ok(ActivityRunResult {
            activity_id: activity.id,
            state: outcome.state,
            duration_ms: outcome.duration_ms,
            error_code: outcome.error_code,
            error_message: outcome.error_message,
        })
    }
}

/// Seed every entry in [`DEFAULT_ACTIVITY_FILES`] as a YAML file under
/// `activities_dir`. Mirrors the skill / executor / policy seeding pattern:
/// the asset YAML is embedded in the binary via `include_str!` and copied
/// out on `orbit init` so the [`V2ActivityCatalog`] can discover it without
/// depending on a git checkout of this repo.
///
/// When `overwrite` is false, existing files are preserved — users who've
/// edited a previously-seeded activity won't lose their changes on re-init.
pub(crate) fn seed_default_activities(
    activities_dir: &Path,
    overwrite: bool,
) -> Result<usize, OrbitError> {
    let mut count = 0usize;
    for (name, content) in DEFAULT_ACTIVITY_FILES {
        let path = activities_dir.join(format!("{name}.yaml"));
        if !overwrite && path.exists() {
            continue;
        }
        write_text_with_parent(&path, content)?;
        count += 1;
    }
    Ok(count)
}

fn validate_activity_params(params: &ActivityAddParams) -> Result<(), OrbitError> {
    if params.id.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "activity id must not be empty".to_string(),
        ));
    }
    if params.spec_type.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "activity type must not be empty".to_string(),
        ));
    }
    if params.description.trim().is_empty() {
        return Err(OrbitError::InvalidInput(
            "activity description must not be empty".to_string(),
        ));
    }
    if !params.input_schema_json.is_object() {
        return Err(OrbitError::InvalidInput(
            "input schema must be a JSON object".to_string(),
        ));
    }
    if !params.output_schema_json.is_object() {
        return Err(OrbitError::InvalidInput(
            "output schema must be a JSON object".to_string(),
        ));
    }
    ensure_spec_config_object(&params.spec_config)?;
    if activity_skill_refs_from_spec_config(&params.spec_config)?
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(OrbitError::InvalidInput(
            "skill_refs must not contain empty values".to_string(),
        ));
    }

    Ok(())
}

fn ensure_spec_config_object(spec_config: &Value) -> Result<(), OrbitError> {
    if spec_config.is_object() {
        Ok(())
    } else {
        Err(OrbitError::InvalidInput(
            "spec_config must be a JSON object".to_string(),
        ))
    }
}

pub(crate) fn activity_requires_agent_cli(spec_type: &str) -> bool {
    spec_type == "agent_invoke"
}

fn validate_activity_executor_for_run(
    runtime: &OrbitRuntime,
    activity: &Activity,
) -> Result<(), OrbitError> {
    let Some(executor_name) = activity
        .executor
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    let executor = runtime
        .stores()
        .executors()
        .get(executor_name)?
        .ok_or_else(|| {
            OrbitError::InvalidInput(format!(
                "activity executor '{}' does not exist",
                executor_name
            ))
        })?;

    if activity.spec_type == "agent_invoke"
        && !matches!(
            executor.executor_type,
            ExecutorType::AgentCli | ExecutorType::DirectAgent
        )
    {
        return Err(OrbitError::InvalidInput(format!(
            "activity executor '{}' has unsupported type '{}' for agent_invoke activities",
            executor_name, executor.executor_type
        )));
    }

    Ok(())
}

fn activity_has_executor(activity: &Activity) -> bool {
    activity
        .executor
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
}

pub(crate) fn activity_skill_refs_from_spec_config(
    spec_config: &Value,
) -> Result<Vec<String>, OrbitError> {
    ensure_spec_config_object(spec_config)?;
    let Some(raw_refs) = spec_config.get("skill_refs") else {
        return Ok(Vec::new());
    };
    serde_json::from_value(raw_refs.clone()).map_err(|error| {
        OrbitError::InvalidInput(format!(
            "activity spec_config.skill_refs must be an array of strings: {error}"
        ))
    })
}
