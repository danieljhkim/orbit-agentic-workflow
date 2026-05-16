use orbit_common::types::{Crew, OrbitError, Task, resolve_crew};
use serde_json::Value;

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn validate_crew_name(&self, crew: Option<&str>) -> Result<(), OrbitError> {
        let Some(crew) = crew.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };
        resolve_crew(crew, self.context.crews()).map(|_| ())
    }

    pub fn resolve_crew_for_task(
        &self,
        cli_override: Option<&str>,
        task_crew: Option<&str>,
    ) -> Result<Crew, OrbitError> {
        let selected = cli_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| task_crew.map(str::trim).filter(|value| !value.is_empty()))
            .or_else(|| self.context.default_crew());
        let Some(selected) = selected else {
            return Err(OrbitError::InvalidInput(
                "no crew selected; set [workflow].default_crew, task.crew, or pass crew"
                    .to_string(),
            ));
        };
        resolve_crew(selected, self.context.crews())
    }

    pub(crate) fn resolve_crew_for_run_input(&self, input: &Value) -> Result<Crew, OrbitError> {
        let cli_override = input
            .get("crew")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let task_crew = self
            .task_id_from_run_input(input)?
            .map(|task| task.crew)
            .unwrap_or_default();
        self.resolve_crew_for_task(cli_override, task_crew.as_deref())
    }

    pub fn resolved_crew_for_task_projection(&self, task: &Task) -> Result<Crew, OrbitError> {
        self.resolve_crew_for_task(None, task.crew.as_deref())
    }

    pub(crate) fn record_run_crew_from_input(
        &self,
        run_id: &str,
        input: &Value,
    ) -> Result<Crew, OrbitError> {
        let crew = self.resolve_crew_for_run_input(input)?;
        tracing::info!(
            run_id,
            resolved_crew = %crew.name,
            planner_model = %crew.planner.model,
            implementer_model = %crew.implementer.model,
            reviewer_model = %crew.reviewer.model,
            "crew resolved for run",
        );
        self.stores().jobs().record_run_crew(run_id, &crew)?;
        Ok(crew)
    }

    fn task_id_from_run_input(&self, input: &Value) -> Result<Option<Task>, OrbitError> {
        for key in ["task_id", "taskId", "id"] {
            let Some(task_id) = input.get(key).and_then(Value::as_str) else {
                continue;
            };
            let task_id = task_id.trim();
            if !task_id.starts_with("ORB-") {
                continue;
            }
            return self.get_task(task_id).map(Some);
        }
        Ok(None)
    }
}
