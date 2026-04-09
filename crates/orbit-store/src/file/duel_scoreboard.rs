//! Append-only log + aggregation for the `duel` evaluation workflow.
//!
//! `.orbit/scoreboard/duel.json` is the single source of truth for the duel
//! evaluation harness. It is a flat file with the shape:
//!
//! ```json
//! { "schema_version": 1, "runs": [ <DuelRun>, ... ] }
//! ```
//!
//! **Design principle:** runs are append-only. Aggregates (per-role/per-agent
//! averages, merge rates, segmented views by `task_class.scope` or
//! `task_class.ambiguity`) are computed on demand from `&[DuelRun]` — nothing
//! is precomputed. This eliminates drift bugs, allows the scoring rubric to
//! evolve without schema migrations, and lets new slicing dimensions be added
//! without rewriting history. Computation is trivial at expected volumes.
//!
//! This module has three public surfaces:
//! - [`append_run`] — writes one run entry, atomically rewriting the file.
//! - [`load_runs`] — reads all run entries back.
//! - [`aggregate`] — pure function over `&[DuelRun]` returning a report.
//! - [`derive_task_scope`] — classifies a diff into `TaskScope`; lives here
//!   because `record_duel_scores` builds the `TaskClass` from git at record
//!   time and the CLI never needs to touch git.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use orbit_types::{Ambiguity, Decision, DuelRun, OrbitError, TaskScope, Verdict};
use serde::{Deserialize, Serialize};

use super::fs_utils::write_atomic;

const SCOREBOARD_FILENAME: &str = "duel.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;

/// On-disk envelope for the scoreboard file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DuelScoreboardFile {
    schema_version: u32,
    #[serde(default)]
    runs: Vec<DuelRun>,
}

impl Default for DuelScoreboardFile {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            runs: Vec::new(),
        }
    }
}

// ============================================================================
// Append + load
// ============================================================================

/// Append a single [`DuelRun`] to `scoreboard_dir/duel.json`, creating the
/// file on first use. Uses the shared atomic-write helper so a crash during
/// the rewrite cannot corrupt earlier entries.
pub fn append_run(scoreboard_dir: &Path, run: &DuelRun) -> Result<(), OrbitError> {
    let path = scoreboard_dir.join(SCOREBOARD_FILENAME);
    let mut file = load_scoreboard_file(&path)?;
    file.runs.push(run.clone());

    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| OrbitError::Io(format!("serialize duel.json: {e}")))?;
    write_atomic(&path, &format!("{json}\n"))
}

/// Load every run entry from `scoreboard_dir/duel.json`. Returns an empty
/// vector if the file does not yet exist.
pub fn load_runs(scoreboard_dir: &Path) -> Result<Vec<DuelRun>, OrbitError> {
    let path = scoreboard_dir.join(SCOREBOARD_FILENAME);
    Ok(load_scoreboard_file(&path)?.runs)
}

fn load_scoreboard_file(path: &Path) -> Result<DuelScoreboardFile, OrbitError> {
    if !path.exists() {
        return Ok(DuelScoreboardFile::default());
    }
    let content =
        fs::read_to_string(path).map_err(|e| OrbitError::Io(format!("read duel.json: {e}")))?;
    if content.trim().is_empty() {
        return Ok(DuelScoreboardFile::default());
    }
    serde_json::from_str(&content).map_err(|e| OrbitError::Io(format!("parse duel.json: {e}")))
}

// ============================================================================
// Task scope derivation (runtime crate discovery, no hardcoded list)
// ============================================================================

/// Derive a [`TaskScope`] for a duel run by diffing `head_ref` against
/// `base_ref` inside `repo_root` and walking the file list against the
/// runtime-discovered crate directories under `orbit/`.
///
/// If `git diff` fails for any reason (missing base ref, repository in a
/// degenerate state, ...) the caller receives a descriptive error rather
/// than a silent default — scope is a signal we do not want to fabricate.
pub fn derive_task_scope(
    repo_root: &Path,
    base_ref: &str,
    head_ref: &str,
) -> Result<TaskScope, OrbitError> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .arg(format!("{base_ref}...{head_ref}"))
        .current_dir(repo_root)
        .output()
        .map_err(|e| OrbitError::Io(format!("spawn git diff: {e}")))?;

    if !output.status.success() {
        return Err(OrbitError::Io(format!(
            "git diff {base_ref}...{head_ref} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim(),
        )));
    }

    let files: Vec<&str> = std::str::from_utf8(&output.stdout)
        .map_err(|e| OrbitError::Io(format!("git diff output not utf-8: {e}")))?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    let crates = discover_crate_dirs(repo_root);
    Ok(classify_scope(&files, &crates))
}

/// Walk `repo_root/crates/` and return the set of crate directory names.
/// A crate directory is any immediate child of `crates/` that contains a
/// `Cargo.toml`. Returned as a `Vec<String>` sorted for determinism.
///
/// This is discovered at runtime precisely so adding a new crate does NOT
/// require touching this module.
fn discover_crate_dirs(repo_root: &Path) -> Vec<String> {
    let crates_dir = repo_root.join("crates");
    let Ok(entries) = fs::read_dir(&crates_dir) else {
        return Vec::new();
    };
    let mut crates: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            if !path.join("Cargo.toml").exists() {
                return None;
            }
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    crates.sort();
    crates
}

/// Classify a set of changed paths into [`TaskScope`].
///
/// Rules (checked in this order):
/// - Empty diff → `Other`.
/// - Exactly one changed file → `SingleFile`.
/// - Two or more files, all bucketed into the same real crate → `MultiFile`.
/// - Two or more files, all bucketed into `Other` (non-crate) → `Other`.
/// - Any mixture of buckets (or 2+ real crates) → `CrossCrate`.
///
/// Non-crate files (`README.md`, `scripts/`, `Makefile`, ...) are bucketed
/// as the synthetic `"other"` key so a README-only diff surfaces as `Other`
/// and a README + crate mix surfaces as `CrossCrate`.
fn classify_scope(files: &[&str], crates: &[String]) -> TaskScope {
    match files.len() {
        0 => TaskScope::Other,
        1 => {
            // Even single-file changes record a scope; callers that want
            // bucket info for aggregation can recompute with more than one.
            TaskScope::SingleFile
        }
        _ => {
            let buckets: BTreeSet<String> =
                files.iter().map(|f| bucket_for_path(f, crates)).collect();
            if buckets.len() == 1 {
                let bucket = buckets.iter().next().expect("len == 1");
                if bucket == "other" {
                    TaskScope::Other
                } else {
                    TaskScope::MultiFile
                }
            } else {
                TaskScope::CrossCrate
            }
        }
    }
}

/// Map a repo-relative path to its bucket name:
/// - `crates/<crate>/...` where `<crate>` is in `crates` → `<crate>`
/// - anything else → `"other"`
///
/// Exposed at module scope for focused unit testing.
fn bucket_for_path(path: &str, crates: &[String]) -> String {
    if let Some(rest) = path.strip_prefix("crates/")
        && let Some(first) = rest.split('/').next()
        && crates.iter().any(|c| c == first)
    {
        return first.to_string();
    }
    "other".to_string()
}

// ============================================================================
// Aggregation (pure function over runs)
// ============================================================================

/// Role axis for aggregation filtering. Mirrors the `--role` CLI flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleAxis {
    Implementer,
    Reviewer,
    Arbiter,
}

/// Segmentation dimension for the `--by` CLI flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SegmentBy {
    #[default]
    None,
    Scope,
    Ambiguity,
}

/// Filter applied by [`aggregate`] before reducing.
#[derive(Debug, Clone, Copy, Default)]
pub struct AggregateFilter {
    /// Restrict to a single role. `None` emits all three.
    pub role: Option<RoleAxis>,
    /// Segment the output along a secondary axis.
    pub segment_by: SegmentBy,
}

/// One row of the aggregated scoreboard view.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AggregateRow {
    /// Segment key; empty string when `segment_by == SegmentBy::None`.
    pub segment: String,
    pub role: &'static str,
    pub agent: String,
    pub model: String,
    pub runs: u32,
    pub avg_score: f64,
    pub merge_rate: f64,
    pub avg_fix_iterations: f64,
    pub avg_wall_seconds: f64,
}

/// Aggregation result. Rows are sorted by (segment, role, agent, model) for
/// deterministic table rendering.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Aggregates {
    pub rows: Vec<AggregateRow>,
}

/// Reduce `runs` into an [`Aggregates`] report.
///
/// This is the only reduction function in the module; the CLI command is a
/// thin wrapper that reads runs via [`load_runs`] and calls this with the
/// flags the user passed. Tests exercise this directly without touching the
/// filesystem.
pub fn aggregate(runs: &[DuelRun], filter: AggregateFilter) -> Aggregates {
    // Bucket key: (segment, role, agent, model)
    #[derive(Default)]
    struct Bucket {
        runs: u32,
        score_sum: f64,
        merged: u32,
        fix_iter_sum: u64,
        wall_sum: u64,
    }

    let mut buckets: BTreeMap<(String, &'static str, String, String), Bucket> = BTreeMap::new();

    let roles_to_emit: &[RoleAxis] = match filter.role {
        Some(RoleAxis::Implementer) => &[RoleAxis::Implementer],
        Some(RoleAxis::Reviewer) => &[RoleAxis::Reviewer],
        Some(RoleAxis::Arbiter) => &[RoleAxis::Arbiter],
        None => &[RoleAxis::Implementer, RoleAxis::Reviewer, RoleAxis::Arbiter],
    };

    for run in runs {
        let segment_key = segment_key_for(run, filter.segment_by);
        for role in roles_to_emit {
            let (role_name, assignment, score) = match role {
                RoleAxis::Implementer => (
                    "implementer",
                    &run.roles.implementer,
                    run.scores.implementer_score as f64,
                ),
                RoleAxis::Reviewer => (
                    "reviewer",
                    &run.roles.reviewer,
                    run.scores.reviewer_score as f64,
                ),
                RoleAxis::Arbiter => {
                    // Arbiter self-assessment is skipped in v1 per the task
                    // spec — there is no arbiter score. Only the run count
                    // and cost fields are meaningful for this role.
                    ("arbiter", &run.roles.arbiter, 0.0)
                }
            };
            let key = (
                segment_key.clone(),
                role_name,
                assignment.agent.clone(),
                assignment.model.clone(),
            );
            let bucket = buckets.entry(key).or_default();
            bucket.runs += 1;
            bucket.score_sum += score;
            if run.outcome.merged {
                bucket.merged += 1;
            }
            bucket.fix_iter_sum += run.outcome.fix_loop_iterations as u64;
            bucket.wall_sum += run.cost.wall_clock_seconds;
        }
    }

    let rows = buckets
        .into_iter()
        .map(|((segment, role, agent, model), b)| {
            let runs = b.runs.max(1) as f64;
            AggregateRow {
                segment,
                role,
                agent,
                model,
                runs: b.runs,
                avg_score: b.score_sum / runs,
                merge_rate: b.merged as f64 / runs,
                avg_fix_iterations: b.fix_iter_sum as f64 / runs,
                avg_wall_seconds: b.wall_sum as f64 / runs,
            }
        })
        .collect();

    Aggregates { rows }
}

fn segment_key_for(run: &DuelRun, axis: SegmentBy) -> String {
    match axis {
        SegmentBy::None => String::new(),
        SegmentBy::Scope => match run.task_class.scope {
            TaskScope::SingleFile => "single_file".into(),
            TaskScope::MultiFile => "multi_file".into(),
            TaskScope::CrossCrate => "cross_crate".into(),
            TaskScope::Other => "other".into(),
        },
        SegmentBy::Ambiguity => match run.task_class.ambiguity {
            Some(Ambiguity::WellSpecified) => "well_specified".into(),
            Some(Ambiguity::NeedsJudgment) => "needs_judgment".into(),
            Some(Ambiguity::Exploratory) => "exploratory".into(),
            None => "unknown".into(),
        },
    }
}

// ============================================================================
// Reviewer-stats helpers (used by record_duel_scores when building DuelRun)
// ============================================================================

/// Tally a list of per-comment verdicts into the four counts plus precision
/// and arbiter_override_rate. Split out so the executor does not duplicate
/// the math and the edge cases (zero comments) stay tested in one place.
pub fn tally_reviewer_stats(verdicts: &[Verdict]) -> ReviewerTally {
    let total = verdicts.len() as u32;
    let mut valid = 0u32;
    let mut invalid = 0u32;
    let mut out_of_scope = 0u32;
    let mut nitpick = 0u32;
    for v in verdicts {
        match v {
            Verdict::Valid => valid += 1,
            Verdict::Invalid => invalid += 1,
            Verdict::OutOfScope => out_of_scope += 1,
            Verdict::Nitpick => nitpick += 1,
        }
    }
    let (precision, override_rate) = if total == 0 {
        (0.0, 0.0)
    } else {
        (
            valid as f64 / total as f64,
            (invalid + out_of_scope + nitpick) as f64 / total as f64,
        )
    };
    ReviewerTally {
        total_comments: total,
        valid,
        invalid,
        out_of_scope,
        nitpick,
        precision,
        arbiter_override_rate: override_rate,
    }
}

/// Output of [`tally_reviewer_stats`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReviewerTally {
    pub total_comments: u32,
    pub valid: u32,
    pub invalid: u32,
    pub out_of_scope: u32,
    pub nitpick: u32,
    pub precision: f64,
    pub arbiter_override_rate: f64,
}

// ============================================================================
// Intentional re-export: the candidate agent family set, so downstream
// callers can reach it via `orbit_store::duel_scoreboard` without a second
// `orbit-types` import. This is cosmetic — `orbit_types::all_agent_families`
// is still the source of truth.
// ============================================================================
pub use orbit_types::all_agent_families as known_agent_families;

// Silence an unused-import lint when the module is consumed by callers that
// do not need the Decision re-export directly.
#[allow(dead_code)]
fn _ensure_decision_in_scope(_: Decision) {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use orbit_types::{
        Ambiguity, Cost, Decision, DuelRun, ImplementerStats, Outcome, ReviewerStats,
        RoleAssignment, Roles, Scores, TaskClass, TaskScope, ValidIssuesBySeverity, Verdict,
    };
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    fn sample_run(run_id: &str, scope: TaskScope, merged: bool, fix_iters: u32) -> DuelRun {
        DuelRun {
            run_id: run_id.into(),
            task_id: "T20260409-0310".into(),
            completed_at: Utc.with_ymd_and_hms(2026, 4, 9, 4, 12, 33).unwrap(),
            task_class: TaskClass {
                scope,
                ambiguity: Some(Ambiguity::WellSpecified),
                source: "derived".into(),
            },
            roles: Roles {
                implementer: RoleAssignment {
                    agent: "claude".into(),
                    model: "opus".into(),
                },
                reviewer: RoleAssignment {
                    agent: "codex".into(),
                    model: "gpt-5.4".into(),
                },
                arbiter: RoleAssignment {
                    agent: "gemini".into(),
                    model: "gemini-3.1-pro".into(),
                },
            },
            outcome: Outcome {
                decision: Decision::Approved,
                fix_loop_iterations: fix_iters,
                fix_loop_exhausted: false,
                pr_number: Some(1),
                merged,
            },
            scores: Scores {
                implementer_score: 4.0,
                reviewer_score: 3.5,
            },
            reviewer_stats: ReviewerStats {
                total_comments: 4,
                valid: 2,
                invalid: 1,
                out_of_scope: 1,
                nitpick: 0,
                precision: 0.5,
                arbiter_override_rate: 0.5,
            },
            implementer_stats: ImplementerStats {
                valid_issues_against: ValidIssuesBySeverity {
                    high: 0,
                    medium: 1,
                    low: 1,
                },
            },
            cost: Cost {
                wall_clock_seconds: 100,
                tokens_in: None,
                tokens_out: None,
            },
        }
    }

    // ---- append / load ----

    #[test]
    fn append_run_creates_file_with_schema_version_and_entry() {
        let dir = tempdir().unwrap();
        let run = sample_run("R1", TaskScope::SingleFile, true, 0);

        append_run(dir.path(), &run).unwrap();

        let contents = fs::read_to_string(dir.path().join(SCOREBOARD_FILENAME)).unwrap();
        let parsed: Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["runs"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["runs"][0]["run_id"], "R1");
    }

    #[test]
    fn append_run_is_append_only_and_preserves_earlier_entries_byte_identical() {
        let dir = tempdir().unwrap();

        let r1 = sample_run("R1", TaskScope::SingleFile, true, 0);
        let r2 = sample_run("R2", TaskScope::MultiFile, false, 2);
        let r3 = sample_run("R3", TaskScope::CrossCrate, true, 1);

        append_run(dir.path(), &r1).unwrap();
        append_run(dir.path(), &r2).unwrap();
        append_run(dir.path(), &r3).unwrap();

        let loaded = load_runs(dir.path()).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].run_id, "R1");
        assert_eq!(loaded[1].run_id, "R2");
        assert_eq!(loaded[2].run_id, "R3");
        assert_eq!(loaded[0], r1);
        assert_eq!(loaded[1], r2);
        // Insertion order + byte-identical earlier entries (verified by
        // round-trip equality — serde_json preserves field order because
        // DuelRun has a fixed struct layout).
        assert_eq!(loaded[2], r3);
    }

    #[test]
    fn load_runs_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let runs = load_runs(dir.path()).unwrap();
        assert!(runs.is_empty());
    }

    // ---- task scope classification ----

    #[test]
    fn classify_scope_empty_is_other() {
        assert_eq!(classify_scope(&[], &[]), TaskScope::Other);
    }

    #[test]
    fn classify_scope_single_file_regardless_of_bucket() {
        let crates = vec!["orbit-types".to_string()];
        assert_eq!(
            classify_scope(&["crates/orbit-types/src/lib.rs"], &crates),
            TaskScope::SingleFile
        );
        assert_eq!(
            classify_scope(&["README.md"], &crates),
            TaskScope::SingleFile
        );
    }

    #[test]
    fn classify_scope_multi_file_same_crate() {
        let crates = vec!["orbit-types".to_string(), "orbit-core".to_string()];
        let files = &[
            "crates/orbit-types/src/lib.rs",
            "crates/orbit-types/src/job.rs",
        ];
        assert_eq!(classify_scope(files, &crates), TaskScope::MultiFile);
    }

    #[test]
    fn classify_scope_cross_crate_for_two_real_crates() {
        let crates = vec!["orbit-types".to_string(), "orbit-core".to_string()];
        let files = &[
            "crates/orbit-types/src/job.rs",
            "crates/orbit-core/src/lib.rs",
        ];
        assert_eq!(classify_scope(files, &crates), TaskScope::CrossCrate);
    }

    #[test]
    fn classify_scope_cross_crate_for_crate_plus_non_crate_file() {
        let crates = vec!["orbit-types".to_string()];
        let files = &["crates/orbit-types/src/job.rs", "README.md"];
        assert_eq!(classify_scope(files, &crates), TaskScope::CrossCrate);
    }

    #[test]
    fn classify_scope_other_when_all_files_outside_crates() {
        let crates = vec!["orbit-types".to_string()];
        let files = &["README.md", "scripts/build.sh", "Makefile"];
        assert_eq!(classify_scope(files, &crates), TaskScope::Other);
    }

    #[test]
    fn bucket_for_path_resolves_known_crates_and_falls_back_to_other() {
        let crates = vec!["orbit-types".to_string(), "orbit-engine".to_string()];
        assert_eq!(
            bucket_for_path("crates/orbit-types/src/lib.rs", &crates),
            "orbit-types"
        );
        assert_eq!(
            bucket_for_path("crates/orbit-engine/src/executor/mod.rs", &crates),
            "orbit-engine"
        );
        // Unknown crate directory under crates/: this is the "new crate added"
        // case — bucketed as `other` here because the walker only knows what
        // it discovered. In production, `discover_crate_dirs` reads the
        // filesystem so new crates are picked up automatically.
        assert_eq!(
            bucket_for_path("crates/orbit-future/src/lib.rs", &crates),
            "other"
        );
        assert_eq!(bucket_for_path("README.md", &crates), "other");
        assert_eq!(bucket_for_path("scripts/ci.sh", &crates), "other");
    }

    #[test]
    fn discover_crate_dirs_auto_picks_up_new_crate_directories() {
        // This test simulates "a new crate is added to crates/" by creating
        // a fake repo layout in a temp dir and asserting the discovery fn
        // walks it without any hardcoded list.
        let dir = tempdir().unwrap();
        let crates_dir = dir.path().join("crates");
        for name in ["orbit-types", "orbit-core", "orbit-future"] {
            let crate_dir = crates_dir.join(name);
            fs::create_dir_all(&crate_dir).unwrap();
            fs::write(crate_dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        }
        // Non-crate directory (no Cargo.toml) — must be ignored.
        fs::create_dir_all(crates_dir.join("scratch")).unwrap();

        let crates = discover_crate_dirs(dir.path());
        assert_eq!(
            crates,
            vec![
                "orbit-core".to_string(),
                "orbit-future".to_string(),
                "orbit-types".to_string(),
            ]
        );
    }

    // ---- reviewer stats math ----

    #[test]
    fn tally_reviewer_stats_zero_comments_is_safe() {
        let t = tally_reviewer_stats(&[]);
        assert_eq!(t.total_comments, 0);
        assert_eq!(t.precision, 0.0);
        assert_eq!(t.arbiter_override_rate, 0.0);
    }

    #[test]
    fn tally_reviewer_stats_mixed_verdicts() {
        // 2 valid, 1 invalid, 1 out_of_scope, 1 nitpick => 5 total
        // precision = 2/5 = 0.4
        // override_rate = 3/5 = 0.6
        let verdicts = [
            Verdict::Valid,
            Verdict::Valid,
            Verdict::Invalid,
            Verdict::OutOfScope,
            Verdict::Nitpick,
        ];
        let t = tally_reviewer_stats(&verdicts);
        assert_eq!(t.total_comments, 5);
        assert_eq!(t.valid, 2);
        assert_eq!(t.invalid, 1);
        assert_eq!(t.out_of_scope, 1);
        assert_eq!(t.nitpick, 1);
        assert!((t.precision - 0.4).abs() < 1e-9);
        assert!((t.arbiter_override_rate - 0.6).abs() < 1e-9);
    }

    #[test]
    fn tally_reviewer_stats_all_valid_gives_zero_override_rate() {
        let verdicts = [Verdict::Valid, Verdict::Valid, Verdict::Valid];
        let t = tally_reviewer_stats(&verdicts);
        assert_eq!(t.precision, 1.0);
        assert_eq!(t.arbiter_override_rate, 0.0);
    }

    // ---- aggregation ----

    #[test]
    fn aggregate_empty_runs_yields_empty_rows() {
        let agg = aggregate(&[], AggregateFilter::default());
        assert!(agg.rows.is_empty());
    }

    #[test]
    fn aggregate_flat_view_has_three_rows_per_run() {
        // One run emits one row per role (implementer/reviewer/arbiter),
        // keyed by (agent, model).
        let runs = vec![sample_run("R1", TaskScope::SingleFile, true, 0)];
        let agg = aggregate(&runs, AggregateFilter::default());
        assert_eq!(agg.rows.len(), 3);
        let roles: Vec<_> = agg.rows.iter().map(|r| r.role).collect();
        assert!(roles.contains(&"implementer"));
        assert!(roles.contains(&"reviewer"));
        assert!(roles.contains(&"arbiter"));
    }

    #[test]
    fn aggregate_computes_merge_rate_and_avg_fix_iterations() {
        let runs = vec![
            sample_run("R1", TaskScope::SingleFile, true, 0),
            sample_run("R2", TaskScope::SingleFile, false, 2),
            sample_run("R3", TaskScope::SingleFile, true, 1),
        ];
        let agg = aggregate(
            &runs,
            AggregateFilter {
                role: Some(RoleAxis::Implementer),
                segment_by: SegmentBy::None,
            },
        );
        assert_eq!(agg.rows.len(), 1); // same implementer across 3 runs
        let row = &agg.rows[0];
        assert_eq!(row.runs, 3);
        // 2 out of 3 merged
        assert!((row.merge_rate - (2.0 / 3.0)).abs() < 1e-9);
        // (0 + 2 + 1) / 3
        assert!((row.avg_fix_iterations - 1.0).abs() < 1e-9);
        assert!((row.avg_score - 4.0).abs() < 1e-9);
        assert!((row.avg_wall_seconds - 100.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_filters_to_single_role() {
        let runs = vec![sample_run("R1", TaskScope::SingleFile, true, 0)];
        let agg = aggregate(
            &runs,
            AggregateFilter {
                role: Some(RoleAxis::Reviewer),
                segment_by: SegmentBy::None,
            },
        );
        assert_eq!(agg.rows.len(), 1);
        assert_eq!(agg.rows[0].role, "reviewer");
        assert_eq!(agg.rows[0].agent, "codex");
    }

    #[test]
    fn aggregate_segments_by_scope() {
        let runs = vec![
            sample_run("R1", TaskScope::SingleFile, true, 0),
            sample_run("R2", TaskScope::MultiFile, true, 0),
            sample_run("R3", TaskScope::SingleFile, false, 0),
        ];
        let agg = aggregate(
            &runs,
            AggregateFilter {
                role: Some(RoleAxis::Implementer),
                segment_by: SegmentBy::Scope,
            },
        );
        // Two segments: single_file (2 runs, 1 merged) and multi_file (1 run, 1 merged)
        let by_segment: BTreeMap<&str, &AggregateRow> =
            agg.rows.iter().map(|r| (r.segment.as_str(), r)).collect();
        let single = by_segment.get("single_file").expect("single_file row");
        assert_eq!(single.runs, 2);
        assert!((single.merge_rate - 0.5).abs() < 1e-9);
        let multi = by_segment.get("multi_file").expect("multi_file row");
        assert_eq!(multi.runs, 1);
        assert!((multi.merge_rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_segments_by_ambiguity_including_unknown_bucket() {
        let mut r1 = sample_run("R1", TaskScope::SingleFile, true, 0);
        r1.task_class.ambiguity = Some(Ambiguity::WellSpecified);
        let mut r2 = sample_run("R2", TaskScope::SingleFile, true, 0);
        r2.task_class.ambiguity = None;
        let mut r3 = sample_run("R3", TaskScope::SingleFile, true, 0);
        r3.task_class.ambiguity = Some(Ambiguity::Exploratory);

        let agg = aggregate(
            &[r1, r2, r3],
            AggregateFilter {
                role: Some(RoleAxis::Implementer),
                segment_by: SegmentBy::Ambiguity,
            },
        );
        let segments: BTreeSet<&str> = agg.rows.iter().map(|r| r.segment.as_str()).collect();
        assert!(segments.contains("well_specified"));
        assert!(segments.contains("unknown"));
        assert!(segments.contains("exploratory"));
    }

    // ---- known_agent_families re-export ----

    #[test]
    fn known_agent_families_matches_orbit_types_source_of_truth() {
        assert_eq!(known_agent_families(), orbit_types::all_agent_families());
    }
}
