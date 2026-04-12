//! Append-only log + aggregation for the planning-duel workflow.
//!
//! `.orbit/scoreboard/duel_plan.json` stores one row per planning duel. Each
//! row captures both planner proposals, the arbiter's winner, and per-role
//! efficiency metrics. Aggregates are computed on demand from the raw rows so
//! the report stays deterministic and schema-light.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use orbit_types::{OrbitError, PlannerSlot, PlanningDuelRun};
use serde::{Deserialize, Serialize};

use super::fs_utils::write_atomic;

const SCOREBOARD_FILENAME: &str = "duel_plan.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;

/// On-disk envelope for the planning-duel scoreboard file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PlanningDuelScoreboardFile {
    schema_version: u32,
    #[serde(default)]
    runs: Vec<PlanningDuelRun>,
}

impl Default for PlanningDuelScoreboardFile {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            runs: Vec::new(),
        }
    }
}

/// The three role axes that can be aggregated independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleAxis {
    PlannerA,
    PlannerB,
    Arbiter,
}

/// Filter applied by [`aggregate`] before reducing.
#[derive(Debug, Clone, Copy, Default)]
pub struct AggregateFilter {
    /// Restrict to a single role. `None` emits all three.
    pub role: Option<RoleAxis>,
}

/// One row of the aggregated planning-duel view.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AggregateRow {
    pub role: &'static str,
    pub agent: String,
    pub model: String,
    pub runs: u32,
    pub points: u32,
    pub avg_wall_seconds: f64,
    pub avg_tool_calls: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_token_total: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_byte_proxy_total: Option<f64>,
}

/// Aggregation result. Rows are sorted by role, agent, and model for
/// deterministic rendering.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Aggregates {
    pub rows: Vec<AggregateRow>,
}

// ============================================================================
// Append + load
// ============================================================================

/// Append a single [`PlanningDuelRun`] to `scoreboard_dir/duel_plan.json`.
pub fn append_run(scoreboard_dir: &Path, run: &PlanningDuelRun) -> Result<(), OrbitError> {
    let path = scoreboard_dir.join(SCOREBOARD_FILENAME);
    let mut file = load_scoreboard_file(&path)?;
    file.runs.push(run.clone());

    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| OrbitError::Io(format!("serialize duel_plan.json: {e}")))?;
    write_atomic(&path, &format!("{json}\n"))
}

/// Load every run entry from `scoreboard_dir/duel_plan.json`.
pub fn load_runs(scoreboard_dir: &Path) -> Result<Vec<PlanningDuelRun>, OrbitError> {
    let path = scoreboard_dir.join(SCOREBOARD_FILENAME);
    Ok(load_scoreboard_file(&path)?.runs)
}

fn load_scoreboard_file(path: &Path) -> Result<PlanningDuelScoreboardFile, OrbitError> {
    if !path.exists() {
        return Ok(PlanningDuelScoreboardFile::default());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| OrbitError::Io(format!("read duel_plan.json: {e}")))?;
    if content.trim().is_empty() {
        return Ok(PlanningDuelScoreboardFile::default());
    }
    serde_json::from_str(&content).map_err(|e| OrbitError::Io(format!("parse duel_plan.json: {e}")))
}

// ============================================================================
// Aggregation
// ============================================================================

/// Reduce `runs` into an [`Aggregates`] report.
pub fn aggregate(runs: &[PlanningDuelRun], filter: AggregateFilter) -> Aggregates {
    #[derive(Default)]
    struct Bucket {
        runs: u32,
        points: u32,
        wall_ms_sum: u128,
        tool_calls_sum: u128,
        token_sum: u128,
        token_count: u32,
        byte_proxy_sum: u128,
        byte_proxy_count: u32,
    }

    let mut buckets: BTreeMap<(String, &'static str, String, String), Bucket> = BTreeMap::new();

    let roles_to_emit: &[RoleAxis] = match filter.role {
        Some(RoleAxis::PlannerA) => &[RoleAxis::PlannerA],
        Some(RoleAxis::PlannerB) => &[RoleAxis::PlannerB],
        Some(RoleAxis::Arbiter) => &[RoleAxis::Arbiter],
        None => &[RoleAxis::PlannerA, RoleAxis::PlannerB, RoleAxis::Arbiter],
    };

    for run in runs {
        for role in roles_to_emit {
            let (role_name, assignment, metrics, points) = match role {
                RoleAxis::PlannerA => (
                    "planner_a",
                    &run.roles.planner_a,
                    &run.efficiency.planner_a,
                    if run.outcome.winner == PlannerSlot::PlannerA {
                        1
                    } else {
                        0
                    },
                ),
                RoleAxis::PlannerB => (
                    "planner_b",
                    &run.roles.planner_b,
                    &run.efficiency.planner_b,
                    if run.outcome.winner == PlannerSlot::PlannerB {
                        1
                    } else {
                        0
                    },
                ),
                RoleAxis::Arbiter => ("arbiter", &run.roles.arbiter, &run.efficiency.arbiter, 0),
            };

            let key = (
                role_name.to_string(),
                role_name,
                assignment.agent.clone(),
                assignment.model.clone(),
            );
            let bucket = buckets.entry(key).or_default();
            bucket.runs += 1;
            bucket.points += points;
            bucket.wall_ms_sum += metrics.wall_clock_ms as u128;
            bucket.tool_calls_sum += metrics.tool_call_count as u128;
            if let Some(total) = metrics.token_total() {
                bucket.token_sum += total as u128;
                bucket.token_count += 1;
            }
            if let Some(total) = metrics.byte_proxy_total() {
                bucket.byte_proxy_sum += total as u128;
                bucket.byte_proxy_count += 1;
            }
        }
    }

    let rows = buckets
        .into_iter()
        .map(|((_, role, agent, model), b)| {
            let runs = b.runs.max(1) as f64;
            AggregateRow {
                role,
                agent,
                model,
                runs: b.runs,
                points: b.points,
                avg_wall_seconds: (b.wall_ms_sum as f64 / runs) / 1_000.0,
                avg_tool_calls: b.tool_calls_sum as f64 / runs,
                avg_token_total: if b.token_count == 0 {
                    None
                } else {
                    Some(b.token_sum as f64 / b.token_count as f64)
                },
                avg_byte_proxy_total: if b.byte_proxy_count == 0 {
                    None
                } else {
                    Some(b.byte_proxy_sum as f64 / b.byte_proxy_count as f64)
                },
            }
        })
        .collect();

    Aggregates { rows }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use orbit_types::{
        EfficiencyMetrics, PlanningDuelRun, PlanningEfficiency, PlanningOutcome,
        PlanningRoleAssignment, PlanningRoles,
    };
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    fn sample_run(run_id: &str, winner: PlannerSlot) -> PlanningDuelRun {
        PlanningDuelRun {
            run_id: run_id.into(),
            task_id: "T20260409-0310".into(),
            completed_at: Utc.with_ymd_and_hms(2026, 4, 9, 4, 12, 33).unwrap(),
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
            planner_a_plan: "Plan A".into(),
            planner_b_plan: "Plan B".into(),
            outcome: PlanningOutcome {
                winner,
                arbiter_rationale: "Plan B has the smaller blast radius.".into(),
            },
            efficiency: PlanningEfficiency {
                planner_a: EfficiencyMetrics {
                    wall_clock_ms: 30_000,
                    tool_call_count: 2,
                    token_usage: Some(orbit_types::TokenUsage {
                        input: 100,
                        cache_read: 0,
                        cache_create: 0,
                        output: 20,
                    }),
                    byte_proxy_total: None,
                },
                planner_b: EfficiencyMetrics {
                    wall_clock_ms: 25_000,
                    tool_call_count: 1,
                    token_usage: None,
                    byte_proxy_total: Some(4096),
                },
                arbiter: EfficiencyMetrics {
                    wall_clock_ms: 8_000,
                    tool_call_count: 0,
                    token_usage: Some(orbit_types::TokenUsage {
                        input: 12,
                        cache_read: 0,
                        cache_create: 0,
                        output: 4,
                    }),
                    byte_proxy_total: None,
                },
            },
        }
    }

    #[test]
    fn append_run_creates_file_with_schema_version_and_entry() {
        let dir = tempdir().unwrap();
        let run = sample_run("P1", PlannerSlot::PlannerB);

        append_run(dir.path(), &run).unwrap();

        let contents = fs::read_to_string(dir.path().join(SCOREBOARD_FILENAME)).unwrap();
        let parsed: Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["runs"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["runs"][0]["run_id"], "P1");
    }

    #[test]
    fn append_run_is_append_only_and_preserves_earlier_entries() {
        let dir = tempdir().unwrap();

        let r1 = sample_run("P1", PlannerSlot::PlannerA);
        let r2 = sample_run("P2", PlannerSlot::PlannerB);

        append_run(dir.path(), &r1).unwrap();
        append_run(dir.path(), &r2).unwrap();

        let loaded = load_runs(dir.path()).unwrap();
        assert_eq!(loaded, vec![r1, r2]);
    }

    #[test]
    fn load_runs_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let runs = load_runs(dir.path()).unwrap();
        assert!(runs.is_empty());
    }

    #[test]
    fn aggregate_awards_exactly_one_point_to_the_winner() {
        let agg = aggregate(
            &[sample_run("P1", PlannerSlot::PlannerB)],
            AggregateFilter::default(),
        );

        let rows: BTreeMap<&str, &AggregateRow> =
            agg.rows.iter().map(|row| (row.role, row)).collect();
        assert_eq!(rows["planner_a"].points, 0);
        assert_eq!(rows["planner_b"].points, 1);
        assert_eq!(rows["arbiter"].points, 0);
        assert_eq!(
            rows.values().map(|row| row.points).sum::<u32>(),
            1,
            "a single planning duel should award exactly one point total"
        );
    }

    #[test]
    fn aggregate_preserves_reportable_efficiency_metrics() {
        let agg = aggregate(
            &[sample_run("P1", PlannerSlot::PlannerA)],
            AggregateFilter {
                role: Some(RoleAxis::PlannerA),
            },
        );

        assert_eq!(agg.rows.len(), 1);
        let row = &agg.rows[0];
        assert_eq!(row.role, "planner_a");
        assert_eq!(row.agent, "claude");
        assert_eq!(row.model, "opus");
        assert_eq!(row.runs, 1);
        assert_eq!(row.points, 1);
        assert!((row.avg_wall_seconds - 30.0).abs() < 1e-9);
        assert!((row.avg_tool_calls - 2.0).abs() < 1e-9);
        assert!((row.avg_token_total.unwrap() - 120.0).abs() < 1e-9);
        assert!(
            row.avg_byte_proxy_total.is_none(),
            "token-backed roles should not invent byte proxies"
        );
    }

    #[test]
    fn aggregate_preserves_byte_proxy_metrics_for_roles_without_exact_tokens() {
        let agg = aggregate(
            &[sample_run("P1", PlannerSlot::PlannerA)],
            AggregateFilter {
                role: Some(RoleAxis::PlannerB),
            },
        );

        assert_eq!(agg.rows.len(), 1);
        let row = &agg.rows[0];
        assert_eq!(row.role, "planner_b");
        assert_eq!(row.points, 0);
        assert!(row.avg_token_total.is_none());
        assert_eq!(row.avg_byte_proxy_total, Some(4096.0));
    }
}
