//! `orbit run duel-plan` CLI entrypoint.

use clap::Args;
use orbit_core::{OrbitError, OrbitRuntime, find_workflow};
use serde_json::{Value, json};

use crate::command::Execute;

use super::support::{dispatch_workflow, print_workflow_dispatch_results};

const DUEL_PLAN_WORKFLOW: &str = "duel-plan";

#[derive(Args)]
#[command(
    about = "Run a planning duel for one task",
    override_usage = "orbit run duel-plan <TASK_ID> [OPTIONS]",
    after_help = "Examples:\n  orbit run duel-plan T20260409-0310\n  orbit run duel-plan T20260409-0310 --base main --json\n  orbit run duel-plan T20260409-0310 --wait\n\nBy default this submits the planning-duel pipeline and returns a run ID immediately. Use `orbit run show <RUN_ID>` to inspect it, or pass `--wait` to block until it finishes."
)]
pub struct DuelPlanCommand {
    /// Task ID for the planning duel.
    pub task_id: String,
    /// Base branch for the planning duel pipeline. Defaults to
    /// `[workflow] base_branch` from `config.toml` (or `main` if unset).
    #[arg(short = 'b', long)]
    pub base: Option<String>,
    /// Wait for the planning-duel pipeline to finish before returning.
    #[arg(long)]
    pub wait: bool,
    /// Output as JSON.
    #[arg(long)]
    pub json: bool,
}

impl Execute for DuelPlanCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let plan = build_duel_plan_run_plan(&self, runtime.workflow_base_branch())?;
        let runs = dispatch_workflow(
            runtime,
            plan.workflow_alias,
            &plan.input,
            false,
            plan.wait_for_completion,
            1,
        )?;
        print_workflow_dispatch_results(plan.workflow_alias, &runs, self.json)
    }
}

#[derive(Debug)]
pub(crate) struct DuelPlanRunPlan {
    pub workflow_alias: &'static str,
    pub input: Value,
    pub wait_for_completion: bool,
}

pub(crate) fn build_duel_plan_run_plan(
    args: &DuelPlanCommand,
    config_base_branch: &str,
) -> Result<DuelPlanRunPlan, OrbitError> {
    find_workflow(DUEL_PLAN_WORKFLOW).ok_or_else(|| {
        OrbitError::InvalidInput(format!("unknown workflow '{DUEL_PLAN_WORKFLOW}'"))
    })?;
    let base = args.base.as_deref().unwrap_or(config_base_branch);
    Ok(DuelPlanRunPlan {
        workflow_alias: DUEL_PLAN_WORKFLOW,
        input: json!({
            "task_id": args.task_id.clone(),
            "task_ids": [args.task_id.clone()],
            "base_branch": base,
        }),
        wait_for_completion: args.wait,
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use orbit_core::OrbitRuntime;
    use serde_json::json;

    use super::*;

    fn duel_plan_args(base: Option<&str>, wait: bool) -> DuelPlanCommand {
        DuelPlanCommand {
            task_id: "T20260425-2010".to_string(),
            base: base.map(str::to_string),
            wait,
            json: false,
        }
    }

    #[test]
    fn duel_plan_uses_explicit_base_when_flag_set() {
        let plan = build_duel_plan_run_plan(&duel_plan_args(Some("main"), false), "agent-main")
            .expect("build duel-plan run plan");

        assert_eq!(plan.workflow_alias, DUEL_PLAN_WORKFLOW);
        assert_eq!(
            plan.input,
            json!({
                "task_id": "T20260425-2010",
                "task_ids": ["T20260425-2010"],
                "base_branch": "main",
            })
        );
    }

    #[test]
    fn duel_plan_falls_back_to_config_base_when_flag_absent() {
        let plan = build_duel_plan_run_plan(&duel_plan_args(None, false), "agent-main")
            .expect("build duel-plan run plan");

        assert_eq!(
            plan.input,
            json!({
                "task_id": "T20260425-2010",
                "task_ids": ["T20260425-2010"],
                "base_branch": "agent-main",
            })
        );
    }

    #[test]
    fn duel_plan_dispatch_defaults_to_non_blocking() {
        let plan = build_duel_plan_run_plan(&duel_plan_args(None, false), "agent-main")
            .expect("build duel-plan run plan");

        assert!(!plan.wait_for_completion);
    }

    #[test]
    fn duel_plan_wait_flag_requests_blocking_dispatch() {
        let plan = build_duel_plan_run_plan(&duel_plan_args(None, true), "agent-main")
            .expect("build duel-plan run plan");

        assert!(plan.wait_for_completion);
    }

    #[test]
    fn default_duel_plan_dispatch_returns_submitted_run_identity() {
        let runtime = OrbitRuntime::in_memory().expect("build runtime");
        let jobs_dir = runtime.data_root().join("resources/jobs");
        std::fs::create_dir_all(&jobs_dir).expect("create jobs dir");
        std::fs::write(
            jobs_dir.join("job_duel_plan_pipeline.yaml"),
            r#"schemaVersion: 2
kind: Job
metadata:
  name: job_duel_plan_pipeline
spec:
  state: enabled
  kind: workflow
  steps:
    - id: marker
      spec:
        type: deterministic
        action: sleep
        config:
          seconds: 0
"#,
        )
        .expect("write job_duel_plan_pipeline fixture");
        let plan = build_duel_plan_run_plan(&duel_plan_args(None, false), "agent-main")
            .expect("build duel-plan run plan");

        let started = Instant::now();
        let runs = dispatch_workflow(
            &runtime,
            plan.workflow_alias,
            &plan.input,
            false,
            plan.wait_for_completion,
            1,
        )
        .expect("dispatch duel-plan workflow");

        assert!(
            started.elapsed() < Duration::from_secs(1),
            "default duel-plan dispatch waited too long"
        );
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].workflow_alias, DUEL_PLAN_WORKFLOW);
        assert_eq!(runs[0].job_id, "job_duel_plan_pipeline");
        assert!(matches!(runs[0].state.as_str(), "submitted" | "queued"));
        assert_eq!(runs[0].attempt, 1);
        assert!(runs[0].error_code.is_none());
        assert!(runs[0].error_message.is_none());
    }
}
