//! Shared schema for the `duel` evaluation workflow.
//!
//! The duel workflow runs a single task end-to-end with a random permutation
//! of agent families assigned to three roles (implementer, reviewer, arbiter)
//! and records structured scores into an append-only run log at
//! `.orbit/scoreboard/duel.json`. The types in this module are the wire
//! contract between:
//!
//! - the `arbitrate_review` activity (which produces an [`ArbiterVerdict`]),
//! - the `check_duel_review_decision` automation (which gates on it),
//! - the `record_duel_scores` automation (which builds a [`DuelRun`] from it),
//! - the `orbit-store` duel_scoreboard module (which persists runs),
//! - the `orbit duel scoreboard` CLI (which computes aggregates over runs).
//!
//! `orbit-types` is the correct home for this module because the engine
//! executors must deserialize [`ArbiterVerdict`] without taking a dependency
//! on `orbit-store`.
//!
//! # Schema evolution
//! [`DuelRun`] uses `#[serde(deny_unknown_fields)]` to catch drift between
//! writer and reader during tests. Any backwards-incompatible field addition
//! must bump `schema_version` on the enclosing scoreboard file and add a
//! migration path in `orbit-store::file::duel_scoreboard`.

use serde::{Deserialize, Serialize};

// ============================================================================
// Arbiter verdict (what `arbitrate_review` emits)
// ============================================================================

/// Structured output of the `arbitrate_review` activity.
///
/// The arbiter independently reads the diff, classifies every reviewer
/// comment, emits two 0–5 scores, and makes the authoritative
/// `APPROVED` / `REQUEST_CHANGES` decision that gates the fix loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArbiterVerdict {
    /// Per-comment classification of every reviewer comment.
    #[serde(default)]
    pub per_comment: Vec<PerCommentVerdict>,
    /// Reviewer quality score on a 0.0–5.0 scale.
    pub reviewer_score: f32,
    /// Implementer quality score on a 0.0–5.0 scale.
    pub implementer_score: f32,
    /// Authoritative decision. Overrides the reviewer's raw output.
    pub decision: Decision,
    /// Subset of `per_comment` whose verdict is `valid`; these are the only
    /// comments that propagate into the next fix-loop iteration.
    #[serde(default)]
    pub blocking_comment_ids: Vec<String>,
    /// Arbiter's classification of task-spec ambiguity. `None` means the
    /// arbiter abstained. This value flows into `DuelRun.task_class.ambiguity`.
    #[serde(default)]
    pub task_class_ambiguity: Option<Ambiguity>,
}

/// One row of the arbiter's per-comment classification table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerCommentVerdict {
    pub comment_id: String,
    pub verdict: Verdict,
    /// Severity applies only to `valid` verdicts; `None` otherwise.
    #[serde(default)]
    pub severity: Option<Severity>,
    pub rationale: String,
}

/// Arbiter's classification of a single reviewer comment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Real issue, within task scope, actionable. Becomes a blocking fix.
    Valid,
    /// Wrong, based on misread, or fabricated.
    Invalid,
    /// Real issue but outside the task's acceptance criteria.
    OutOfScope,
    /// Stylistic or trivial, not worth blocking.
    Nitpick,
}

/// Severity weighting for `valid` comments; used to compute implementer score.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// Authoritative decision output by the arbiter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    Approved,
    RequestChanges,
}

/// Arbiter's classification of task-spec ambiguity.
///
/// The arbiter is already reading the task spec + diff to validate reviewer
/// comments, so it is the cheapest point at which to classify ambiguity.
/// This flows into `DuelRun.task_class.ambiguity` unchanged.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Ambiguity {
    /// Requirements fully determine the correct implementation.
    WellSpecified,
    /// Requirements leave room for reasonable judgment calls.
    NeedsJudgment,
    /// Requirements are exploratory; multiple valid end-states exist.
    Exploratory,
}

// ============================================================================
// DuelRun (what `record_duel_scores` appends to the scoreboard)
// ============================================================================

/// One row in `.orbit/scoreboard/duel.json` — the append-only source of truth
/// for duel evaluation results.
///
/// Aggregates (per-agent averages, merge rates, segmented views) are computed
/// on demand from `&[DuelRun]` by the `orbit-store` scoreboard module; nothing
/// is precomputed. This eliminates drift bugs and lets the rubric evolve
/// without schema migrations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DuelRun {
    pub run_id: String,
    pub task_id: String,
    /// Wall-clock timestamp at which `record_duel_scores` wrote this entry.
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub task_class: TaskClass,
    pub roles: Roles,
    pub outcome: Outcome,
    pub scores: Scores,
    pub reviewer_stats: ReviewerStats,
    pub implementer_stats: ImplementerStats,
    pub cost: Cost,
}

/// Classification dimensions used to segment scoreboard aggregates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TaskClass {
    /// Derived by `record_duel_scores` from `git diff --name-only <base>...<head>`.
    pub scope: TaskScope,
    /// Copied verbatim from [`ArbiterVerdict::task_class_ambiguity`]. `None`
    /// preserves the arbiter's abstention.
    #[serde(default)]
    pub ambiguity: Option<Ambiguity>,
    /// In v1 always `"derived"` (scope from git, ambiguity from arbiter).
    /// Reserved for future values like `"human"` or `"override"`.
    pub source: String,
}

/// Scope classification derived from the diff file list.
///
/// Path classification walks prefixes against the runtime-discovered list
/// of crate directories under `orbit/`; non-crate files (README, scripts,
/// Makefile, top-level config) are bucketed as `Other`. A diff mixing
/// `Other` with any real crate is treated as `CrossCrate`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskScope {
    /// Exactly one file changed.
    SingleFile,
    /// Two or more files changed, all within a single crate.
    MultiFile,
    /// Files touched span two or more crates (or a mix of crate and non-crate).
    CrossCrate,
    /// All changed files lie outside any known crate directory.
    Other,
}

/// Agent/model assignment for one role in a duel run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct RoleAssignment {
    pub agent: String,
    pub model: String,
}

/// The three role assignments for a duel run. Always all-distinct on the
/// `agent` field by construction (see `select_duel_roles`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Roles {
    pub implementer: RoleAssignment,
    pub reviewer: RoleAssignment,
    pub arbiter: RoleAssignment,
}

/// Pipeline outcome for one duel run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    /// Final arbiter decision at the time the run was recorded. In the
    /// fix-loop-exhausted path this is the decision from the last iteration.
    pub decision: Decision,
    /// Number of fix-loop iterations that executed (0 if approved on first review).
    pub fix_loop_iterations: u32,
    /// True iff the loop hit its `max_iterations` cap without converging.
    pub fix_loop_exhausted: bool,
    /// GitHub PR number associated with this run. `None` if PR creation failed.
    #[serde(default)]
    pub pr_number: Option<u64>,
    /// True iff the PR was merged (distinct from `decision: Approved` because
    /// an approved PR can still fail to merge due to CI flakes or conflicts).
    pub merged: bool,
}

/// Final scores emitted by the arbiter for the run's reviewer and implementer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scores {
    pub implementer_score: f32,
    pub reviewer_score: f32,
}

/// Aggregate statistics about the reviewer's comment stream for this run.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReviewerStats {
    pub total_comments: u32,
    pub valid: u32,
    pub invalid: u32,
    pub out_of_scope: u32,
    pub nitpick: u32,
    /// `valid / total_comments`, or `0.0` when `total_comments == 0`.
    /// Stored as f64 for serde precision — arithmetic is trivial.
    pub precision: f64,
    /// Fraction of reviewer comments the arbiter did NOT classify as `valid`:
    /// `(invalid + out_of_scope + nitpick) / total_comments`, or `0.0` when
    /// `total_comments == 0`. Named "override" because in practice every
    /// reviewer comment is posted as a blocker — so any non-`valid` verdict
    /// is effectively the arbiter overriding the reviewer.
    pub arbiter_override_rate: f64,
}

/// Count of `valid` issues the arbiter confirmed in the diff, bucketed by
/// severity. Drives the implementer score rubric.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImplementerStats {
    pub valid_issues_against: ValidIssuesBySeverity,
}

/// Severity histogram for confirmed issues against the implementer.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidIssuesBySeverity {
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

/// Resource cost of the run. v1 populates only wall-clock; token fields
/// are reserved and always `None` until the agent runtime starts reporting
/// them end-to-end.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cost {
    pub wall_clock_seconds: u64,
    #[serde(default)]
    pub tokens_in: Option<u64>,
    #[serde(default)]
    pub tokens_out: Option<u64>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- ArbiterVerdict ----

    #[test]
    fn arbiter_verdict_round_trips_with_all_enum_values() {
        let verdict = ArbiterVerdict {
            per_comment: vec![
                PerCommentVerdict {
                    comment_id: "c1".into(),
                    verdict: Verdict::Valid,
                    severity: Some(Severity::High),
                    rationale: "Null pointer deref on line 42".into(),
                },
                PerCommentVerdict {
                    comment_id: "c2".into(),
                    verdict: Verdict::Invalid,
                    severity: None,
                    rationale: "Misread the diff; function is private".into(),
                },
                PerCommentVerdict {
                    comment_id: "c3".into(),
                    verdict: Verdict::OutOfScope,
                    severity: None,
                    rationale: "Real bug but in a file outside acceptance criteria".into(),
                },
                PerCommentVerdict {
                    comment_id: "c4".into(),
                    verdict: Verdict::Nitpick,
                    severity: None,
                    rationale: "Style preference".into(),
                },
            ],
            reviewer_score: 3.5,
            implementer_score: 4.0,
            decision: Decision::RequestChanges,
            blocking_comment_ids: vec!["c1".into()],
            task_class_ambiguity: Some(Ambiguity::NeedsJudgment),
        };

        let json = serde_json::to_string(&verdict).expect("serialize");
        let parsed: ArbiterVerdict = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, verdict);
    }

    #[test]
    fn arbiter_verdict_ambiguity_accepts_all_four_values_including_null() {
        for (raw, expected) in [
            (json!("well_specified"), Some(Ambiguity::WellSpecified)),
            (json!("needs_judgment"), Some(Ambiguity::NeedsJudgment)),
            (json!("exploratory"), Some(Ambiguity::Exploratory)),
            (json!(null), None),
        ] {
            let verdict_json = json!({
                "per_comment": [],
                "reviewer_score": 5.0,
                "implementer_score": 5.0,
                "decision": "APPROVED",
                "blocking_comment_ids": [],
                "task_class_ambiguity": raw,
            });
            let parsed: ArbiterVerdict =
                serde_json::from_value(verdict_json).expect("deserialize");
            assert_eq!(parsed.task_class_ambiguity, expected);
        }
    }

    #[test]
    fn arbiter_verdict_missing_task_class_ambiguity_defaults_to_none() {
        let raw = json!({
            "per_comment": [],
            "reviewer_score": 5.0,
            "implementer_score": 5.0,
            "decision": "APPROVED",
            "blocking_comment_ids": [],
        });
        let parsed: ArbiterVerdict = serde_json::from_value(raw).expect("deserialize");
        assert_eq!(parsed.task_class_ambiguity, None);
    }

    #[test]
    fn decision_serializes_as_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&Decision::Approved).unwrap(),
            "\"APPROVED\""
        );
        assert_eq!(
            serde_json::to_string(&Decision::RequestChanges).unwrap(),
            "\"REQUEST_CHANGES\""
        );
    }

    // ---- DuelRun ----

    fn sample_run() -> DuelRun {
        DuelRun {
            run_id: "R20260409-0310-001".into(),
            task_id: "T20260409-0310".into(),
            completed_at: "2026-04-09T04:12:33Z".parse().unwrap(),
            task_class: TaskClass {
                scope: TaskScope::MultiFile,
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
                fix_loop_iterations: 2,
                fix_loop_exhausted: false,
                pr_number: Some(142),
                merged: true,
            },
            scores: Scores {
                implementer_score: 4.0,
                reviewer_score: 3.5,
            },
            reviewer_stats: ReviewerStats {
                total_comments: 6,
                valid: 3,
                invalid: 1,
                out_of_scope: 1,
                nitpick: 1,
                precision: 0.5,
                arbiter_override_rate: 0.5,
            },
            implementer_stats: ImplementerStats {
                valid_issues_against: ValidIssuesBySeverity {
                    high: 0,
                    medium: 1,
                    low: 2,
                },
            },
            cost: Cost {
                wall_clock_seconds: 812,
                tokens_in: None,
                tokens_out: None,
            },
        }
    }

    #[test]
    fn duel_run_round_trips_every_documented_field() {
        let run = sample_run();
        let json = serde_json::to_string(&run).expect("serialize");
        let parsed: DuelRun = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, run);
    }

    #[test]
    fn duel_run_rejects_unknown_top_level_field() {
        let mut raw = serde_json::to_value(sample_run()).unwrap();
        raw.as_object_mut()
            .unwrap()
            .insert("future_field".into(), json!("surprise"));
        let err = serde_json::from_value::<DuelRun>(raw).unwrap_err();
        assert!(
            err.to_string().contains("future_field"),
            "deny_unknown_fields should mention the unknown field: {err}"
        );
    }

    #[test]
    fn task_scope_other_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskScope::Other).unwrap(),
            "\"other\""
        );
        assert_eq!(
            serde_json::to_string(&TaskScope::CrossCrate).unwrap(),
            "\"cross_crate\""
        );
    }
}
