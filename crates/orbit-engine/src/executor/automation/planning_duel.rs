//! Planning-duel automation helpers.
//!
//! This module stays intentionally small:
//! - role selection picks two distinct planners plus one arbiter
//! - task writeback persists the winning plan and a backlog history entry
//! - efficiency recording replays persisted invocation traces by role

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use orbit_store::{InvocationRecord, planning_duel_scoreboard};
use orbit_types::{
    EfficiencyMetrics, OrbitError, PlannerSlot, PlanningDuelRun, PlanningEfficiency,
    PlanningOutcome, PlanningRoleAssignment, PlanningRoles, TaskComment, TaskStatus, TokenUsage,
    all_agent_families, resolve_agent_model_pair,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};

use super::input::{input_string_field, required_input_string};

const PERMUTATIONS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

thread_local! {
    static TEST_PERMUTATION_QUEUE: RefCell<VecDeque<[usize; 3]>> =
        const { RefCell::new(VecDeque::new()) };
}

#[cfg(test)]
pub(crate) fn push_test_permutations(perms: impl IntoIterator<Item = [usize; 3]>) {
    TEST_PERMUTATION_QUEUE.with(|cell| {
        cell.borrow_mut().extend(perms);
    });
}

#[cfg(test)]
pub(crate) fn clear_test_permutations() {
    TEST_PERMUTATION_QUEUE.with(|cell| cell.borrow_mut().clear());
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlanningDuelRoleAssignment {
    agent: String,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlanningDuelRoleMetrics {
    agent: String,
    model: String,
    activity_id: String,
    efficiency: PlanningDuelEfficiency,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct PlanningDuelEfficiency {
    invocation_count: u64,
    wall_clock_ms: u64,
    tool_call_count: u64,
    input_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    byte_proxy_total: u64,
}

fn next_permutation() -> [usize; 3] {
    let from_test = TEST_PERMUTATION_QUEUE.with(|cell| cell.borrow_mut().pop_front());
    if let Some(perm) = from_test {
        return perm;
    }

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    PERMUTATIONS[(nanos as usize) % PERMUTATIONS.len()]
}

fn orchestrator_model_for(family: &str) -> Result<String, OrbitError> {
    resolve_agent_model_pair(family)
        .map(|pair| pair.orchestrator)
        .ok_or_else(|| {
            OrbitError::Execution(format!(
                "no registered model pair for agent family '{family}'"
            ))
        })
}

fn build_role_assignment(family: &str) -> Result<PlanningDuelRoleAssignment, OrbitError> {
    Ok(PlanningDuelRoleAssignment {
        agent: family.to_string(),
        model: orchestrator_model_for(family)?,
    })
}

fn build_roles_output(perm: [usize; 3]) -> Result<Value, OrbitError> {
    let families = all_agent_families();
    let planner_a = families[perm[0]];
    let planner_b = families[perm[1]];
    let arbiter = families[perm[2]];

    if planner_a == planner_b || planner_b == arbiter || planner_a == arbiter {
        return Err(OrbitError::Execution(format!(
            "select_planning_duel_roles produced non-distinct families: \
             planner_a={planner_a}, planner_b={planner_b}, arbiter={arbiter}"
        )));
    }

    let started_at = Utc::now().to_rfc3339();

    Ok(json!({
        "planner_a_agent_cli": planner_a,
        "planner_a_model": orchestrator_model_for(planner_a)?,
        "planner_b_agent_cli": planner_b,
        "planner_b_model": orchestrator_model_for(planner_b)?,
        "arbiter_agent_cli": arbiter,
        "arbiter_model": orchestrator_model_for(arbiter)?,
        "planning_duel_started_at": started_at,
        "planning_duel_roles": {
            "planner_a": build_role_assignment(planner_a)?,
            "planner_b": build_role_assignment(planner_b)?,
            "arbiter": build_role_assignment(arbiter)?,
        }
    }))
}

fn role_activity_id(input: &Value, role: &str) -> Result<String, OrbitError> {
    let flat_key = format!("{role}_activity_id");
    if let Some(value) = input_string_field(input, &flat_key) {
        return Ok(value);
    }

    input
        .get("roles")
        .and_then(|roles| roles.get(role))
        .and_then(|entry| {
            entry
                .get("activity_id")
                .and_then(Value::as_str)
                .or_else(|| entry.get("activityId").and_then(Value::as_str))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| OrbitError::InvalidInput(format!("missing required input.{flat_key}")))
}

fn role_identity(input: &Value, role: &str) -> Result<PlanningDuelRoleAssignment, OrbitError> {
    let agent_key = format!("{role}_agent_cli");
    let model_key = format!("{role}_model");
    let agent = required_input_string(input, &agent_key)?.to_string();
    let model = required_input_string(input, &model_key)?.to_string();
    Ok(PlanningDuelRoleAssignment { agent, model })
}

fn summarize_role_metrics(records: &[InvocationRecord]) -> PlanningDuelEfficiency {
    let mut efficiency = PlanningDuelEfficiency {
        invocation_count: records.len() as u64,
        ..PlanningDuelEfficiency::default()
    };

    for record in records {
        efficiency.wall_clock_ms = efficiency.wall_clock_ms.saturating_add(record.duration_ms);
        efficiency.tool_call_count = efficiency
            .tool_call_count
            .saturating_add(record.tool_call_count);
        efficiency.input_tokens = efficiency.input_tokens.saturating_add(record.input_tokens);
        efficiency.cache_read_tokens = efficiency
            .cache_read_tokens
            .saturating_add(record.cache_read_tokens);
        efficiency.cache_create_tokens = efficiency
            .cache_create_tokens
            .saturating_add(record.cache_create_tokens);
        efficiency.output_tokens = efficiency
            .output_tokens
            .saturating_add(record.output_tokens);
        efficiency.total_tokens = efficiency.total_tokens.saturating_add(record.total_tokens);
        efficiency.byte_proxy_total = efficiency.byte_proxy_total.saturating_add(
            record
                .tool_calls
                .iter()
                .map(|tool_call| tool_call.result_bytes)
                .sum::<u64>(),
        );
    }

    efficiency
}

fn role_metrics_for_activity<H: RuntimeHost + ?Sized>(
    host: &H,
    job_run_id: &str,
    role_id: &PlanningDuelRoleAssignment,
    activity_id: &str,
) -> Result<PlanningDuelRoleMetrics, OrbitError> {
    let records = host.invocation_records_for_job_run_and_activity(job_run_id, activity_id)?;
    for record in &records {
        if record.agent != role_id.agent || record.model.as_deref() != Some(role_id.model.as_str())
        {
            return Err(OrbitError::Execution(format!(
                "activity '{activity_id}' for job run '{job_run_id}' returned invocation \
                 attributed to {}/{:?} instead of expected {}/{}",
                record.agent, record.model, role_id.agent, role_id.model
            )));
        }
    }

    Ok(PlanningDuelRoleMetrics {
        agent: role_id.agent.clone(),
        model: role_id.model.clone(),
        activity_id: activity_id.to_string(),
        efficiency: summarize_role_metrics(&records),
    })
}

fn ensure_distinct_activity_ids(activity_ids: [(&str, &str); 3]) -> Result<(), OrbitError> {
    for left in 0..activity_ids.len() {
        for right in (left + 1)..activity_ids.len() {
            if activity_ids[left].1 == activity_ids[right].1 {
                return Err(OrbitError::InvalidInput(format!(
                    "{} and {} must use distinct activity ids (both were '{}')",
                    activity_ids[left].0, activity_ids[right].0, activity_ids[left].1
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn select_planning_duel_roles(input: &Value) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let perm = next_permutation();
    let output = build_roles_output(perm)?;

    Ok(json!({
        "task_id": task_id,
        "planning_duel_started_at": output["planning_duel_started_at"].clone(),
        "planner_a_agent_cli": output["planner_a_agent_cli"].clone(),
        "planner_a_model": output["planner_a_model"].clone(),
        "planner_b_agent_cli": output["planner_b_agent_cli"].clone(),
        "planner_b_model": output["planner_b_model"].clone(),
        "arbiter_agent_cli": output["arbiter_agent_cli"].clone(),
        "arbiter_model": output["arbiter_model"].clone(),
        "planning_duel_roles": output["planning_duel_roles"].clone(),
    }))
}

pub(super) fn writeback_planning_duel_task<H: TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let winning_plan = required_input_string(input, "winning_plan")?.to_string();
    let winner_role = required_input_string(input, "winner_role")?;
    let winner = role_identity(input, "winner")?;
    let arbiter = role_identity(input, "arbiter")?;
    let rationale = required_input_string(input, "arbiter_rationale")?.to_string();

    let status_note = format!(
        "planning duel winner={winner_role} ({}/{})",
        winner.agent, winner.model
    );
    let comment_message = format!(
        "Planning duel resolved.\n\nWinner: {winner_role} ({}/{})\n\nRationale: {rationale}\n\nWinning plan persisted to task.plan.",
        winner.agent, winner.model
    );

    host.apply_task_automation_update(
        task_id,
        TaskAutomationUpdate {
            plan: Some(winning_plan),
            status: Some(TaskStatus::Backlog),
            status_event: Some("planning_duel_resolved".to_string()),
            status_note: Some(format!("{status_note}; rationale={rationale}")),
            append_comments: vec![TaskComment {
                at: Utc::now(),
                by: arbiter.agent.clone(),
                message: comment_message,
            }],
            agent: Some(winner.agent.clone()),
            model: Some(winner.model.clone()),
            ..TaskAutomationUpdate::default()
        },
    )?;

    Ok(json!({
        "task_id": task_id,
        "status": "backlog",
        "winner_role": winner_role,
        "winner_agent_cli": winner.agent,
        "winner_model": winner.model,
        "arbiter_agent_cli": arbiter.agent,
        "arbiter_model": arbiter.model,
    }))
}

pub(super) fn record_planning_duel_efficiency<H: RuntimeHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let job_run_id = input_string_field(input, "job_run_id")
        .or_else(|| input_string_field(input, "run_id"))
        .ok_or_else(|| OrbitError::InvalidInput("missing required input.run_id".to_string()))?;

    let planner_a = role_identity(input, "planner_a")?;
    let planner_b = role_identity(input, "planner_b")?;
    let arbiter = role_identity(input, "arbiter")?;

    let planner_a_activity_id = role_activity_id(input, "planner_a")?;
    let planner_b_activity_id = role_activity_id(input, "planner_b")?;
    let arbiter_activity_id = role_activity_id(input, "arbiter")?;
    ensure_distinct_activity_ids([
        ("planner_a_activity_id", &planner_a_activity_id),
        ("planner_b_activity_id", &planner_b_activity_id),
        ("arbiter_activity_id", &arbiter_activity_id),
    ])?;

    let planner_a_metrics =
        role_metrics_for_activity(host, &job_run_id, &planner_a, &planner_a_activity_id)?;
    let planner_b_metrics =
        role_metrics_for_activity(host, &job_run_id, &planner_b, &planner_b_activity_id)?;
    let arbiter_metrics =
        role_metrics_for_activity(host, &job_run_id, &arbiter, &arbiter_activity_id)?;

    let roles = BTreeMap::from([
        ("planner_a".to_string(), planner_a_metrics),
        ("planner_b".to_string(), planner_b_metrics),
        ("arbiter".to_string(), arbiter_metrics),
    ]);

    Ok(json!({
        "task_id": task_id,
        "job_run_id": job_run_id,
        "roles": roles,
    }))
}

fn into_role_assignment(value: PlanningDuelRoleAssignment) -> PlanningRoleAssignment {
    PlanningRoleAssignment {
        agent: value.agent,
        model: value.model,
    }
}

fn into_efficiency_metrics(value: PlanningDuelEfficiency) -> EfficiencyMetrics {
    let token_usage = TokenUsage {
        input: value.input_tokens,
        cache_read: value.cache_read_tokens,
        cache_create: value.cache_create_tokens,
        output: value.output_tokens,
    };
    let has_exact_tokens = token_usage.input > 0
        || token_usage.cache_read > 0
        || token_usage.cache_create > 0
        || token_usage.output > 0;

    EfficiencyMetrics {
        wall_clock_ms: value.wall_clock_ms,
        tool_call_count: value.tool_call_count.min(u32::MAX as u64) as u32,
        token_usage: has_exact_tokens.then_some(token_usage),
        byte_proxy_total: (!has_exact_tokens && value.byte_proxy_total > 0)
            .then_some(value.byte_proxy_total),
    }
}

pub(super) fn record_planning_duel_scores<H: RuntimeHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let task_id = required_input_string(input, "task_id")?;
    let job_run_id = input_string_field(input, "job_run_id")
        .or_else(|| input_string_field(input, "run_id"))
        .ok_or_else(|| OrbitError::InvalidInput("missing required input.job_run_id".to_string()))?;
    let winner_role = required_input_string(input, "winner_role")?;
    let winner = match winner_role {
        "planner_a" => PlannerSlot::PlannerA,
        "planner_b" => PlannerSlot::PlannerB,
        other => {
            return Err(OrbitError::InvalidInput(format!(
                "winner_role must be 'planner_a' or 'planner_b', got '{other}'"
            )));
        }
    };

    let planner_a_role = serde_json::from_value::<PlanningDuelRoleMetrics>(
        input
            .get("roles")
            .and_then(|roles| roles.get("planner_a"))
            .cloned()
            .ok_or_else(|| {
                OrbitError::InvalidInput("missing required input.roles.planner_a".to_string())
            })?,
    )
    .map_err(|err| OrbitError::InvalidInput(format!("invalid roles.planner_a payload: {err}")))?;
    let planner_b_role = serde_json::from_value::<PlanningDuelRoleMetrics>(
        input
            .get("roles")
            .and_then(|roles| roles.get("planner_b"))
            .cloned()
            .ok_or_else(|| {
                OrbitError::InvalidInput("missing required input.roles.planner_b".to_string())
            })?,
    )
    .map_err(|err| OrbitError::InvalidInput(format!("invalid roles.planner_b payload: {err}")))?;
    let arbiter_role = serde_json::from_value::<PlanningDuelRoleMetrics>(
        input
            .get("roles")
            .and_then(|roles| roles.get("arbiter"))
            .cloned()
            .ok_or_else(|| {
                OrbitError::InvalidInput("missing required input.roles.arbiter".to_string())
            })?,
    )
    .map_err(|err| OrbitError::InvalidInput(format!("invalid roles.arbiter payload: {err}")))?;

    let PlanningDuelRoleMetrics {
        agent: planner_a_agent,
        model: planner_a_model,
        activity_id: _,
        efficiency: planner_a_efficiency,
    } = planner_a_role;
    let PlanningDuelRoleMetrics {
        agent: planner_b_agent,
        model: planner_b_model,
        activity_id: _,
        efficiency: planner_b_efficiency,
    } = planner_b_role;
    let PlanningDuelRoleMetrics {
        agent: arbiter_agent,
        model: arbiter_model,
        activity_id: _,
        efficiency: arbiter_efficiency,
    } = arbiter_role;

    let completed_at = Utc::now();
    let run = PlanningDuelRun {
        run_id: job_run_id,
        task_id: task_id.to_string(),
        completed_at,
        roles: PlanningRoles {
            planner_a: into_role_assignment(PlanningDuelRoleAssignment {
                agent: planner_a_agent,
                model: planner_a_model,
            }),
            planner_b: into_role_assignment(PlanningDuelRoleAssignment {
                agent: planner_b_agent,
                model: planner_b_model,
            }),
            arbiter: into_role_assignment(PlanningDuelRoleAssignment {
                agent: arbiter_agent,
                model: arbiter_model,
            }),
        },
        planner_a_plan: required_input_string(input, "planner_a_proposal")?.to_string(),
        planner_b_plan: required_input_string(input, "planner_b_proposal")?.to_string(),
        outcome: PlanningOutcome {
            winner,
            arbiter_rationale: required_input_string(input, "arbiter_rationale")?.to_string(),
        },
        efficiency: PlanningEfficiency {
            planner_a: into_efficiency_metrics(planner_a_efficiency),
            planner_b: into_efficiency_metrics(planner_b_efficiency),
            arbiter: into_efficiency_metrics(arbiter_efficiency),
        },
    };

    planning_duel_scoreboard::append_run(host.scoreboard_dir(), &run)?;

    Ok(json!({
        "run_id": run.run_id,
        "recorded": true,
    }))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use chrono::{DateTime, Utc};
    use orbit_store::{
        InvocationQuery, InvocationRecord, InvocationToolCallRecord, planning_duel_scoreboard,
    };
    use orbit_types::{OrbitError, TaskStatus};
    use serde_json::Value;
    use serde_json::json;
    use tempfile::tempdir;

    use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};

    use super::{
        PlanningDuelEfficiency, clear_test_permutations, push_test_permutations,
        record_planning_duel_efficiency, record_planning_duel_scores, select_planning_duel_roles,
        summarize_role_metrics, writeback_planning_duel_task,
    };

    #[derive(Default)]
    struct StubTaskHost {
        updates: Mutex<Vec<(String, TaskAutomationUpdate)>>,
    }

    impl TaskHost for StubTaskHost {
        fn get_task(&self, _task_id: &str) -> Result<orbit_types::Task, OrbitError> {
            unimplemented!("not used")
        }

        fn list_tasks_filtered(
            &self,
            _status: Option<orbit_types::TaskStatus>,
            _priority: Option<orbit_types::TaskPriority>,
            _parent_id: Option<&str>,
            _batch_id: Option<&str>,
        ) -> Result<Vec<orbit_types::Task>, OrbitError> {
            Ok(Vec::new())
        }

        fn start_task(
            &self,
            _task_id: &str,
            _note: Option<String>,
            _comment: Option<String>,
        ) -> Result<orbit_types::Task, OrbitError> {
            unimplemented!()
        }

        fn update_task_from_activity(
            &self,
            _task_id: &str,
            _status: orbit_types::TaskStatus,
            _execution_summary: Option<String>,
            _comment: Option<String>,
            _note: Option<String>,
        ) -> Result<orbit_types::Task, OrbitError> {
            unimplemented!()
        }

        fn apply_task_automation_update(
            &self,
            task_id: &str,
            update: TaskAutomationUpdate,
        ) -> Result<(), OrbitError> {
            self.updates
                .lock()
                .unwrap()
                .push((task_id.to_string(), update));
            Ok(())
        }
    }

    struct StubRuntimeHost {
        scoreboard_dir: tempfile::TempDir,
        queries: Mutex<Vec<(String, String)>>,
        records: Mutex<HashMap<(String, String), Vec<InvocationRecord>>>,
    }

    impl StubRuntimeHost {
        fn new() -> Self {
            Self {
                scoreboard_dir: tempdir().expect("tempdir"),
                queries: Mutex::new(Vec::new()),
                records: Mutex::new(HashMap::new()),
            }
        }
    }

    impl RuntimeHost for StubRuntimeHost {
        fn record_event(&self, _event: orbit_types::OrbitEvent) -> Result<(), OrbitError> {
            unimplemented!()
        }

        fn repo_root(&self) -> Result<String, OrbitError> {
            unimplemented!()
        }

        fn data_root(&self) -> &std::path::Path {
            unimplemented!()
        }

        fn run_job_now_with_input_debug(
            &self,
            _job_id: &str,
            _input: Value,
            _debug: bool,
        ) -> Result<crate::JobRunResult, OrbitError> {
            unimplemented!()
        }

        fn validate_activity_target_exists(
            &self,
            _target_type: orbit_types::JobTargetType,
            _target_id: &str,
        ) -> Result<orbit_types::Activity, OrbitError> {
            unimplemented!()
        }

        fn get_job(&self, _job_id: &str) -> Result<Option<orbit_types::Job>, OrbitError> {
            unimplemented!()
        }

        fn invocation_records(
            &self,
            query: InvocationQuery,
        ) -> Result<Vec<InvocationRecord>, OrbitError> {
            let job_run_id = query.job_run_id.unwrap_or_default();
            let activity_id = query.activity_id.unwrap_or_default();
            self.queries
                .lock()
                .unwrap()
                .push((job_run_id.clone(), activity_id.clone()));
            Ok(self
                .records
                .lock()
                .unwrap()
                .get(&(job_run_id, activity_id))
                .cloned()
                .unwrap_or_default())
        }

        fn run_tool_with_context_and_role(
            &self,
            _name: &str,
            _input: Value,
            _role: orbit_types::Role,
            _tool_context: orbit_tools::ToolContext,
        ) -> Result<Value, OrbitError> {
            unimplemented!()
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
            unimplemented!()
        }

        fn scoring_enabled(&self) -> bool {
            false
        }

        fn graph_editing(&self) -> bool {
            false
        }

        fn scoreboard_dir(&self) -> &std::path::Path {
            self.scoreboard_dir.path()
        }
    }

    fn record(
        id: i64,
        ts: &str,
        job_run_id: &str,
        activity_id: &str,
        agent: &str,
        model: &str,
        duration_ms: u64,
        input_tokens: u64,
        output_tokens: u64,
        tool_calls: &[(&str, u64)],
    ) -> InvocationRecord {
        InvocationRecord {
            id,
            ts: DateTime::parse_from_rfc3339(ts)
                .unwrap()
                .with_timezone(&Utc),
            job_run_id: job_run_id.to_string(),
            activity_id: activity_id.to_string(),
            agent: agent.to_string(),
            model: Some(model.to_string()),
            duration_ms,
            input_tokens,
            cache_read_tokens: 0,
            cache_create_tokens: 0,
            output_tokens,
            total_tokens: input_tokens.saturating_add(output_tokens),
            tool_call_count: tool_calls.len() as u64,
            task_ids: vec!["T1".to_string()],
            tool_calls: tool_calls
                .iter()
                .enumerate()
                .map(
                    |(seq, (tool_name, result_bytes))| InvocationToolCallRecord {
                        invocation_id: id,
                        seq: seq as u64 + 1,
                        tool_name: tool_name.to_string(),
                        result_bytes: *result_bytes,
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn select_planning_duel_roles_returns_distinct_assignments() {
        clear_test_permutations();
        push_test_permutations([[0, 1, 2]]);

        let out = select_planning_duel_roles(&json!({ "task_id": "T1" })).unwrap();
        assert_eq!(out["planner_a_agent_cli"], json!("codex"));
        assert_eq!(out["planner_b_agent_cli"], json!("claude"));
        assert_eq!(out["arbiter_agent_cli"], json!("gemini"));
        assert_eq!(
            out["planning_duel_roles"]["planner_a"]["agent"],
            json!("codex")
        );
        assert_eq!(
            out["planning_duel_roles"]["arbiter"]["agent"],
            json!("gemini")
        );
    }

    #[test]
    fn writeback_planning_duel_task_persists_plan_backlog_and_commentary() {
        let host = StubTaskHost::default();
        let out = writeback_planning_duel_task(
            &host,
            &json!({
                "task_id": "T1",
                "winning_plan": "new plan",
                "winner_role": "planner_a",
                "winner_agent_cli": "codex",
                "winner_model": "gpt-5.4",
                "arbiter_agent_cli": "gemini",
                "arbiter_model": "gemini-3.1-pro",
                "arbiter_rationale": "best plan",
            }),
        )
        .unwrap();

        assert_eq!(out["status"], json!("backlog"));
        let updates = host.updates.lock().unwrap();
        assert_eq!(updates.len(), 1);
        let update = &updates[0].1;
        assert_eq!(update.plan.as_deref(), Some("new plan"));
        assert_eq!(update.status, Some(TaskStatus::Backlog));
        assert_eq!(
            update.status_event.as_deref(),
            Some("planning_duel_resolved")
        );
        assert_eq!(update.agent.as_deref(), Some("codex"));
        assert_eq!(update.append_comments.len(), 1);
        assert_eq!(update.append_comments[0].by, "gemini");
    }

    #[test]
    fn record_planning_duel_efficiency_queries_invocation_traces_by_role() {
        let host = StubRuntimeHost::new();
        host.records.lock().unwrap().insert(
            ("run-1".to_string(), "activity-a".to_string()),
            vec![
                record(
                    1,
                    "2026-04-11T12:00:00Z",
                    "run-1",
                    "activity-a",
                    "codex",
                    "gpt-5.4",
                    25,
                    10,
                    3,
                    &[("orbit.graph.search", 40), ("orbit.graph.refs", 20)],
                ),
                record(
                    2,
                    "2026-04-11T12:01:00Z",
                    "run-1",
                    "activity-a",
                    "codex",
                    "gpt-5.4",
                    35,
                    5,
                    7,
                    &[("orbit.graph.show", 50)],
                ),
            ],
        );
        host.records.lock().unwrap().insert(
            ("run-1".to_string(), "activity-b".to_string()),
            vec![record(
                3,
                "2026-04-11T12:02:00Z",
                "run-1",
                "activity-b",
                "claude",
                "opus",
                60,
                12,
                8,
                &[("orbit.graph.overview", 60)],
            )],
        );
        host.records.lock().unwrap().insert(
            ("run-1".to_string(), "activity-c".to_string()),
            vec![record(
                4,
                "2026-04-11T12:03:00Z",
                "run-1",
                "activity-c",
                "gemini",
                "gemini-3.1-pro",
                90,
                1,
                2,
                &[("orbit.graph.search", 70)],
            )],
        );

        let out = record_planning_duel_efficiency(
            &host,
            &json!({
                "task_id": "T1",
                "job_run_id": "run-1",
                "planner_a_agent_cli": "codex",
                "planner_a_model": "gpt-5.4",
                "planner_a_activity_id": "activity-a",
                "planner_b_agent_cli": "claude",
                "planner_b_model": "opus",
                "planner_b_activity_id": "activity-b",
                "arbiter_agent_cli": "gemini",
                "arbiter_model": "gemini-3.1-pro",
                "arbiter_activity_id": "activity-c",
            }),
        )
        .unwrap();

        assert_eq!(out["task_id"], json!("T1"));
        assert_eq!(
            out["roles"]["planner_a"]["efficiency"]["invocation_count"],
            json!(2)
        );
        assert_eq!(
            out["roles"]["planner_a"]["efficiency"]["wall_clock_ms"],
            json!(60)
        );
        assert_eq!(
            out["roles"]["planner_a"]["efficiency"]["tool_call_count"],
            json!(3)
        );
        assert_eq!(
            out["roles"]["planner_a"]["efficiency"]["byte_proxy_total"],
            json!(110)
        );
        assert_eq!(
            out["roles"]["planner_b"]["efficiency"]["byte_proxy_total"],
            json!(60)
        );
        assert_eq!(
            out["roles"]["arbiter"]["efficiency"]["wall_clock_ms"],
            json!(90)
        );
        assert_eq!(host.queries.lock().unwrap().len(), 3);
    }

    #[test]
    fn record_planning_duel_scores_appends_scoreboard_entry() {
        let host = StubRuntimeHost::new();

        let out = record_planning_duel_scores(
            &host,
            &json!({
                "job_run_id": "run-77",
                "task_id": "T1",
                "planner_a_proposal": "## Plan\n- planner a",
                "planner_b_proposal": "## Plan\n- planner b",
                "winner_role": "planner_b",
                "arbiter_rationale": "planner_b grounded the writeback path more clearly",
                "roles": {
                    "planner_a": {
                        "agent": "codex",
                        "model": "gpt-5.4",
                        "activity_id": "propose_duel_plan_a",
                        "efficiency": {
                            "invocation_count": 1,
                            "wall_clock_ms": 1200,
                            "tool_call_count": 3,
                            "input_tokens": 20,
                            "cache_read_tokens": 0,
                            "cache_create_tokens": 0,
                            "output_tokens": 10,
                            "total_tokens": 30,
                            "byte_proxy_total": 0
                        }
                    },
                    "planner_b": {
                        "agent": "claude",
                        "model": "opus",
                        "activity_id": "propose_duel_plan_b",
                        "efficiency": {
                            "invocation_count": 1,
                            "wall_clock_ms": 1500,
                            "tool_call_count": 4,
                            "input_tokens": 0,
                            "cache_read_tokens": 0,
                            "cache_create_tokens": 0,
                            "output_tokens": 0,
                            "total_tokens": 0,
                            "byte_proxy_total": 4096
                        }
                    },
                    "arbiter": {
                        "agent": "gemini",
                        "model": "gemini-3.1-pro",
                        "activity_id": "arbitrate_duel_plan",
                        "efficiency": {
                            "invocation_count": 1,
                            "wall_clock_ms": 900,
                            "tool_call_count": 2,
                            "input_tokens": 15,
                            "cache_read_tokens": 0,
                            "cache_create_tokens": 0,
                            "output_tokens": 5,
                            "total_tokens": 20,
                            "byte_proxy_total": 0
                        }
                    }
                }
            }),
        )
        .unwrap();

        assert_eq!(out["recorded"], json!(true));
        let runs = planning_duel_scoreboard::load_runs(host.scoreboard_dir()).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, "run-77");
        assert_eq!(runs[0].task_id, "T1");
        assert_eq!(runs[0].outcome.winner, orbit_types::PlannerSlot::PlannerB);
        assert_eq!(runs[0].roles.planner_b.agent, "claude");
        assert_eq!(runs[0].planner_b_plan, "## Plan\n- planner b");
        assert_eq!(runs[0].efficiency.planner_a.token_total(), Some(30));
        assert_eq!(runs[0].efficiency.planner_b.byte_proxy_total(), Some(4096));
    }

    #[test]
    fn record_planning_duel_efficiency_rejects_duplicate_activity_ids() {
        let host = StubRuntimeHost::new();

        let err = record_planning_duel_efficiency(
            &host,
            &json!({
                "task_id": "T1",
                "job_run_id": "run-1",
                "planner_a_agent_cli": "codex",
                "planner_a_model": "gpt-5.4",
                "planner_a_activity_id": "shared-activity",
                "planner_b_agent_cli": "claude",
                "planner_b_model": "opus",
                "planner_b_activity_id": "shared-activity",
                "arbiter_agent_cli": "gemini",
                "arbiter_model": "gemini-3.1-pro",
                "arbiter_activity_id": "activity-c",
            }),
        )
        .unwrap_err();

        assert!(err.to_string().contains("must use distinct activity ids"));
    }

    #[test]
    fn record_planning_duel_efficiency_rejects_misattributed_invocations() {
        let host = StubRuntimeHost::new();
        host.records.lock().unwrap().insert(
            ("run-1".to_string(), "activity-a".to_string()),
            vec![record(
                1,
                "2026-04-11T12:00:00Z",
                "run-1",
                "activity-a",
                "claude",
                "opus",
                25,
                10,
                3,
                &[("orbit.graph.search", 40)],
            )],
        );

        let err = record_planning_duel_efficiency(
            &host,
            &json!({
                "task_id": "T1",
                "job_run_id": "run-1",
                "planner_a_agent_cli": "codex",
                "planner_a_model": "gpt-5.4",
                "planner_a_activity_id": "activity-a",
                "planner_b_agent_cli": "claude",
                "planner_b_model": "opus",
                "planner_b_activity_id": "activity-b",
                "arbiter_agent_cli": "gemini",
                "arbiter_model": "gemini-3.1-pro",
                "arbiter_activity_id": "activity-c",
            }),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("instead of expected codex/gpt-5.4")
        );
    }

    #[test]
    fn summarize_role_metrics_accumulates_totals() {
        let metrics = summarize_role_metrics(&[
            record(
                1,
                "2026-04-11T12:00:00Z",
                "run-1",
                "activity-a",
                "codex",
                "gpt-5.4",
                10,
                4,
                2,
                &[("tool-a", 5)],
            ),
            record(
                2,
                "2026-04-11T12:00:01Z",
                "run-1",
                "activity-a",
                "codex",
                "gpt-5.4",
                20,
                6,
                8,
                &[("tool-b", 7)],
            ),
        ]);

        assert_eq!(
            metrics,
            PlanningDuelEfficiency {
                invocation_count: 2,
                wall_clock_ms: 30,
                tool_call_count: 2,
                input_tokens: 10,
                cache_read_tokens: 0,
                cache_create_tokens: 0,
                output_tokens: 10,
                total_tokens: 20,
                byte_proxy_total: 12,
            }
        );
    }
}
