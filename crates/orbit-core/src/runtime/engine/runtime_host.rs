use orbit_common::types::{
    Activity, AgentModelPair, InvocationTrace, JobTargetType, OrbitError, OrbitEvent, Role,
};
use orbit_engine::{ActivityInvocationResult, ExecutionContext, RuntimeHost};
use orbit_store::{InvocationInsertParams, InvocationQuery, InvocationRecord, token_scoreboard};
use orbit_tools::ToolContext;
use serde_json::Value;

use super::identity::normalize_agent_name;
use super::invocation::{associated_task_ids, open_invocation_store};
use super::paths::current_repo_root;
use crate::OrbitRuntime;

impl RuntimeHost for OrbitRuntime {
    fn record_event(&self, event: OrbitEvent) -> Result<(), OrbitError> {
        OrbitRuntime::record_event(self, event)
    }

    fn repo_root(&self) -> Result<String, OrbitError> {
        current_repo_root(self)
    }

    fn data_root(&self) -> &std::path::Path {
        self.context.data_root()
    }

    fn activity_executor_registry(&self) -> &orbit_engine::ActivityExecutorRegistry {
        self.activity_executor_registry()
    }

    fn run_job_now_with_input_debug(
        &self,
        job_id: &str,
        _input: Value,
        _debug: bool,
    ) -> Result<orbit_engine::JobRunResult, OrbitError> {
        Err(OrbitError::Execution(format!(
            "v1 job dispatch is retired; refusing to run job '{job_id}' via RuntimeHost"
        )))
    }

    fn validate_activity_target_exists(
        &self,
        _target_type: JobTargetType,
        target_id: &str,
    ) -> Result<Activity, OrbitError> {
        Err(OrbitError::Execution(format!(
            "v1 activity lookup is retired; refusing to resolve activity '{target_id}'"
        )))
    }

    fn get_job(&self, _job_id: &str) -> Result<Option<orbit_common::types::Job>, OrbitError> {
        Ok(None)
    }

    fn resolved_agent_model_pair(&self, agent_cli: &str) -> Option<AgentModelPair> {
        self.configured_agent_model_pair(agent_cli)
    }

    fn canonical_model_name(&self, agent_cli: &str, model: Option<&str>) -> Option<String> {
        self.canonical_model_for_agent(agent_cli, model)
    }

    fn invocation_records(
        &self,
        query: InvocationQuery,
    ) -> Result<Vec<InvocationRecord>, OrbitError> {
        OrbitRuntime::invocation_records(self, query)
    }

    fn run_tool_with_context_and_role(
        &self,
        name: &str,
        input: Value,
        role: Role,
        tool_context: ToolContext,
    ) -> Result<Value, OrbitError> {
        OrbitRuntime::run_tool_with_context_and_role(self, name, input, role, tool_context)
    }

    fn invoke_activity(
        &self,
        activity: Activity,
        _agent_cli: &str,
        _model: Option<&str>,
        _input: Value,
        _timeout_seconds: u64,
        _debug: bool,
    ) -> Result<ActivityInvocationResult, OrbitError> {
        Err(OrbitError::Execution(format!(
            "v1 invoke_activity is retired; activity '{}' cannot be dispatched via RuntimeHost",
            activity.id
        )))
    }

    fn maybe_create_failure_task(
        &self,
        _job_id: &str,
        _run_id: &str,
        _error_code: &str,
        _error_message: &str,
        _agent: Option<&str>,
        _model: Option<&str>,
    ) -> Result<(), OrbitError> {
        Ok(())
    }

    fn scoring_enabled(&self) -> bool {
        self.context.scoring_enabled()
    }

    fn graph_editing(&self) -> bool {
        self.context.graph_editing()
    }

    fn scoreboard_dir(&self) -> &std::path::Path {
        &self.context.paths().scoreboard_dir
    }

    fn persist_invocation_trace(
        &self,
        job_run_id: &str,
        execution: &ExecutionContext,
        trace: &InvocationTrace,
    ) -> Result<(), OrbitError> {
        let requested_model = execution
            .model
            .as_deref()
            .or(execution.model_tier.as_deref());
        let (agent, model) =
            self.canonical_agent_model_identity(Some(&execution.agent_cli), requested_model);
        let store = open_invocation_store(self)?;
        store.insert_invocation_trace_record(&InvocationInsertParams {
            job_run_id: job_run_id.to_string(),
            activity_id: execution.activity.id.clone(),
            agent: agent.unwrap_or_else(|| normalize_agent_name(&execution.agent_cli)),
            model,
            task_ids: associated_task_ids(&execution.input),
            trace: trace.clone(),
        })?;

        if let Err(error) =
            token_scoreboard::write_token_scoreboard(&self.paths().scoreboard_dir, &store)
        {
            eprintln!("orbit: failed to refresh tokens scoreboard: {error}");
        }

        Ok(())
    }
}
