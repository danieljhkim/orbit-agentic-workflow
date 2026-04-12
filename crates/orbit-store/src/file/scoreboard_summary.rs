use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use orbit_types::{OrbitError, PlannerSlot, Task, TaskStatus};
use serde::{Deserialize, Serialize};

use super::fs_utils::write_atomic;
use super::planning_duel_scoreboard;

const SUMMARY_FILENAME: &str = "summary.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;

type NestedScoreboard = BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrictionSummary {
    pub reported: u64,
    pub accepted: u64,
    pub rejected: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenSummary {
    pub total: u64,
    pub output: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DuelSummary {
    pub wins: u64,
    pub losses: u64,
    pub participated: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrSummary {
    pub review_comments: u64,
    pub merged_clean: u64,
    pub merged_with_revision: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSummary {
    pub tasks_completed: u64,
    pub friction: FrictionSummary,
    pub tokens: TokenSummary,
    pub duels: DuelSummary,
    pub pr: PrSummary,
    pub tool_calls: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScoreboardSummary {
    pub schema_version: u32,
    pub generated_at: String,
    pub agents: BTreeMap<String, AgentSummary>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TokenScoreboardFile {
    #[serde(default)]
    agents: Vec<TokenAgentEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TokenAgentEntry {
    agent: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    total_tokens: u64,
    #[serde(default, alias = "output_tokens")]
    total_output_tokens: u64,
    #[serde(default)]
    total_tool_calls: u64,
}

pub fn generate_summary(
    scoreboard_dir: &Path,
    tasks: &[Task],
) -> Result<ScoreboardSummary, OrbitError> {
    let mut agents: BTreeMap<String, AgentSummary> = BTreeMap::new();

    let friction = read_nested_scoreboard(scoreboard_dir, "friction_bounty.json")?;
    overlay_nested_metric(
        &mut agents,
        &friction,
        "issues-reported",
        |summary, value| {
            summary.friction.reported = summary.friction.reported.saturating_add(value);
        },
    );
    overlay_nested_metric(
        &mut agents,
        &friction,
        "issues-accepted",
        |summary, value| {
            summary.friction.accepted = summary.friction.accepted.saturating_add(value);
        },
    );
    overlay_nested_metric(
        &mut agents,
        &friction,
        "issues-rejected",
        |summary, value| {
            summary.friction.rejected = summary.friction.rejected.saturating_add(value);
        },
    );

    let pr = read_nested_scoreboard(scoreboard_dir, "pr.json")?;
    overlay_nested_metric(&mut agents, &pr, "pr-review-comments", |summary, value| {
        summary.pr.review_comments = summary.pr.review_comments.saturating_add(value);
    });
    overlay_nested_metric(
        &mut agents,
        &pr,
        "pr-count-without-revision",
        |summary, value| {
            summary.pr.merged_clean = summary.pr.merged_clean.saturating_add(value);
        },
    );
    overlay_nested_metric(
        &mut agents,
        &pr,
        "pr-count-with-revision",
        |summary, value| {
            summary.pr.merged_with_revision = summary.pr.merged_with_revision.saturating_add(value);
        },
    );

    for token_row in read_token_agents(scoreboard_dir)? {
        let summary = agents
            .entry(agent_key(&token_row.agent, &token_row.model))
            .or_default();
        summary.tokens.total = summary.tokens.total.saturating_add(token_row.total_tokens);
        summary.tokens.output = summary
            .tokens
            .output
            .saturating_add(token_row.total_output_tokens);
        summary.tool_calls = summary
            .tool_calls
            .saturating_add(token_row.total_tool_calls);
    }

    for run in planning_duel_scoreboard::load_runs(scoreboard_dir)? {
        let planner_a = agents
            .entry(agent_key(
                &run.roles.planner_a.agent,
                &run.roles.planner_a.model,
            ))
            .or_default();
        planner_a.duels.participated = planner_a.duels.participated.saturating_add(1);
        let planner_b = agents
            .entry(agent_key(
                &run.roles.planner_b.agent,
                &run.roles.planner_b.model,
            ))
            .or_default();
        planner_b.duels.participated = planner_b.duels.participated.saturating_add(1);
        let arbiter = agents
            .entry(agent_key(
                &run.roles.arbiter.agent,
                &run.roles.arbiter.model,
            ))
            .or_default();
        arbiter.duels.participated = arbiter.duels.participated.saturating_add(1);

        match run.outcome.winner {
            PlannerSlot::PlannerA => {
                let planner_a = agents
                    .entry(agent_key(
                        &run.roles.planner_a.agent,
                        &run.roles.planner_a.model,
                    ))
                    .or_default();
                planner_a.duels.wins = planner_a.duels.wins.saturating_add(1);
                let planner_b = agents
                    .entry(agent_key(
                        &run.roles.planner_b.agent,
                        &run.roles.planner_b.model,
                    ))
                    .or_default();
                planner_b.duels.losses = planner_b.duels.losses.saturating_add(1);
            }
            PlannerSlot::PlannerB => {
                let planner_b = agents
                    .entry(agent_key(
                        &run.roles.planner_b.agent,
                        &run.roles.planner_b.model,
                    ))
                    .or_default();
                planner_b.duels.wins = planner_b.duels.wins.saturating_add(1);
                let planner_a = agents
                    .entry(agent_key(
                        &run.roles.planner_a.agent,
                        &run.roles.planner_a.model,
                    ))
                    .or_default();
                planner_a.duels.losses = planner_a.duels.losses.saturating_add(1);
            }
        }
    }

    for task in tasks {
        if !matches!(task.status, TaskStatus::Done | TaskStatus::Archived) {
            continue;
        }
        let Some(agent) = task.actor_identity.agent_name() else {
            continue;
        };
        let Some(model) = task.actor_identity.agent_model() else {
            continue;
        };
        let summary = agents.entry(agent_key(agent, model)).or_default();
        summary.tasks_completed = summary.tasks_completed.saturating_add(1);
    }

    Ok(ScoreboardSummary {
        schema_version: CURRENT_SCHEMA_VERSION,
        generated_at: Utc::now().to_rfc3339(),
        agents,
    })
}

pub fn write_summary(
    scoreboard_dir: &Path,
    summary: &ScoreboardSummary,
) -> Result<std::path::PathBuf, OrbitError> {
    let path = scoreboard_dir.join(SUMMARY_FILENAME);
    let raw = serde_json::to_string_pretty(summary)
        .map_err(|e| OrbitError::Io(format!("serialize summary.json: {e}")))?;
    write_atomic(&path, &format!("{raw}\n"))?;
    Ok(path)
}

pub fn summary_path(scoreboard_dir: &Path) -> std::path::PathBuf {
    scoreboard_dir.join(SUMMARY_FILENAME)
}

fn read_nested_scoreboard(
    scoreboard_dir: &Path,
    file_name: &str,
) -> Result<NestedScoreboard, OrbitError> {
    let path = scoreboard_dir.join(file_name);
    if !path.exists() {
        return Ok(NestedScoreboard::new());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| OrbitError::Io(format!("read {file_name}: {e}")))?;
    if raw.trim().is_empty() {
        return Ok(NestedScoreboard::new());
    }
    serde_json::from_str(&raw).map_err(|e| OrbitError::Io(format!("parse {file_name}: {e}")))
}

fn read_token_agents(scoreboard_dir: &Path) -> Result<Vec<TokenAgentEntry>, OrbitError> {
    let path = scoreboard_dir.join("tokens.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| OrbitError::Io(format!("read tokens.json: {e}")))?;
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let parsed: TokenScoreboardFile = serde_json::from_str(&raw)
        .map_err(|e| OrbitError::Io(format!("parse tokens.json: {e}")))?;
    Ok(parsed.agents)
}

fn overlay_nested_metric(
    agents: &mut BTreeMap<String, AgentSummary>,
    scoreboard: &NestedScoreboard,
    metric: &str,
    mut apply: impl FnMut(&mut AgentSummary, u64),
) {
    let Some(by_agent) = scoreboard.get(metric) else {
        return;
    };

    for (agent, by_model) in by_agent {
        for (model, value) in by_model {
            let summary = agents.entry(agent_key(agent, model)).or_default();
            apply(summary, *value);
        }
    }
}

fn agent_key(agent: &str, model: &str) -> String {
    format!("{}/{}", agent.trim(), model.trim())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::Utc;
    use orbit_types::{
        ActorIdentity, EfficiencyMetrics, PlannerSlot, PlanningDuelRun, PlanningEfficiency,
        PlanningOutcome, PlanningRoleAssignment, PlanningRoles, Task, TaskPriority, TaskStatus,
        TaskType,
    };
    use tempfile::tempdir;

    use super::{generate_summary, write_summary};
    use crate::file::planning_duel_scoreboard;

    #[test]
    fn generate_summary_aggregates_scoreboards_and_tasks() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("friction_bounty.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "issues-reported": { "codex": { "gpt-5.4": 2 } },
                "issues-accepted": { "codex": { "gpt-5.4": 1 } },
                "issues-rejected": { "claude": { "opus": 1 } }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            dir.path().join("pr.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "pr-review-comments": { "claude": { "opus": 3 } },
                "pr-count-without-revision": { "codex": { "gpt-5.4": 1 } },
                "pr-count-with-revision": { "claude": { "opus": 2 } }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            dir.path().join("tokens.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "agents": [
                    {
                        "agent": "codex",
                        "model": "gpt-5.4",
                        "total_tokens": 120,
                        "total_output_tokens": 45,
                        "total_tool_calls": 8
                    },
                    {
                        "agent": "claude",
                        "model": "opus",
                        "total_tokens": 90,
                        "total_output_tokens": 20,
                        "total_tool_calls": 4
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        planning_duel_scoreboard::append_run(
            dir.path(),
            &PlanningDuelRun {
                run_id: "run-1".into(),
                task_id: "T1".into(),
                completed_at: Utc::now(),
                roles: PlanningRoles {
                    planner_a: PlanningRoleAssignment {
                        agent: "claude".into(),
                        model: "opus".into(),
                    },
                    planner_b: PlanningRoleAssignment {
                        agent: "codex".into(),
                        model: "gpt-5.4".into(),
                    },
                    arbiter: PlanningRoleAssignment {
                        agent: "gemini".into(),
                        model: "gemini-3.1-pro-preview".into(),
                    },
                },
                planner_a_artifact_path: "planning-duel/claude-opus.md".into(),
                planner_b_artifact_path: "planning-duel/codex-gpt-5.4.md".into(),
                outcome: PlanningOutcome {
                    winner: PlannerSlot::PlannerA,
                    arbiter_rationale: "Plan A wins".into(),
                },
                efficiency: PlanningEfficiency {
                    planner_a: EfficiencyMetrics {
                        wall_clock_ms: 10,
                        tool_call_count: 1,
                        token_usage: None,
                        byte_proxy_total: Some(11),
                    },
                    planner_b: EfficiencyMetrics {
                        wall_clock_ms: 20,
                        tool_call_count: 2,
                        token_usage: None,
                        byte_proxy_total: Some(22),
                    },
                    arbiter: EfficiencyMetrics {
                        wall_clock_ms: 30,
                        tool_call_count: 0,
                        token_usage: None,
                        byte_proxy_total: None,
                    },
                },
            },
        )
        .unwrap();

        let tasks = vec![
            Task {
                id: "T-done-1".into(),
                parent_id: None,
                title: "done".into(),
                description: String::new(),
                acceptance_criteria: vec![],
                plan: String::new(),
                execution_summary: String::new(),
                context_files: vec![],
                workspace_path: None,
                repo_root: None,
                assigned_to: Some("codex / gpt-5.4".into()),
                created_by: None,
                actor_identity: ActorIdentity::agent("codex", "gpt-5.4"),
                status: TaskStatus::Done,
                priority: TaskPriority::Medium,
                complexity: None,
                task_type: TaskType::Task,
                pr_number: None,
                pr_status: None,
                proposed_by: None,
                source_task_id: None,
                batch_id: None,
                comments: vec![],
                history: vec![],
                review_threads: vec![],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Task {
                id: "T-open".into(),
                parent_id: None,
                title: "open".into(),
                description: String::new(),
                acceptance_criteria: vec![],
                plan: String::new(),
                execution_summary: String::new(),
                context_files: vec![],
                workspace_path: None,
                repo_root: None,
                assigned_to: Some("claude / opus".into()),
                created_by: None,
                actor_identity: ActorIdentity::agent("claude", "opus"),
                status: TaskStatus::Backlog,
                priority: TaskPriority::Medium,
                complexity: None,
                task_type: TaskType::Task,
                pr_number: None,
                pr_status: None,
                proposed_by: None,
                source_task_id: None,
                batch_id: None,
                comments: vec![],
                history: vec![],
                review_threads: vec![],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let summary = generate_summary(dir.path(), &tasks).expect("summary");
        assert_eq!(summary.schema_version, 1);
        assert_eq!(summary.agents["codex/gpt-5.4"].tasks_completed, 1);
        assert_eq!(summary.agents["codex/gpt-5.4"].friction.reported, 2);
        assert_eq!(summary.agents["codex/gpt-5.4"].pr.merged_clean, 1);
        assert_eq!(summary.agents["codex/gpt-5.4"].tokens.total, 120);
        assert_eq!(summary.agents["codex/gpt-5.4"].tool_calls, 8);
        assert_eq!(summary.agents["codex/gpt-5.4"].duels.losses, 1);
        assert_eq!(summary.agents["claude/opus"].duels.wins, 1);
        assert_eq!(
            summary.agents["gemini/gemini-3.1-pro-preview"]
                .duels
                .participated,
            1
        );

        let path = write_summary(dir.path(), &summary).expect("write");
        assert!(path.ends_with("summary.json"));
        assert!(dir.path().join("summary.json").exists());
    }
}
