# Orbit CEO Assessment
**Author:** Steve (CEO)
**Date:** 2026-03-09

---

## Current Strengths

Orbit has the right bones. Three invariants make it worth building on:

1. **No hidden mutation.** Every state change produces an event and an audit record. This is the single most important decision in the codebase. Don't dilute it.
2. **Hermetic execution environment.** `inherit = false` by default is correct. Most automation runtimes fail because they inherit ambient state. Orbit refuses that trap.
3. **Explicit job runtime.** `orbit job serve` as a foreground process rather than a daemon means there is no mystery about when the system is running. Simple is right.

The skill system is well-conceived. Behavioral contracts as markdown files are human-readable, version-controllable, and machine-loadable. That's the right tradeoff.

---

## Key Problems

### 1. Token bloat is an existential problem for agent usability

`feature.md` already documents this clearly: a single activity session consumes ~14k tokens because full task objects—plan, execution summary, description, context files—are returned even for operational queries. This makes the system expensive to operate autonomously at scale.

The design answer (signal / summary / artifact) is already articulated. It is not implemented.

**This is the highest-priority fix in the codebase.**

### 2. CLI taxonomy inconsistency: `job-run` is a first-class command

`orbit job-run` lives as a top-level CLI command separate from `orbit job`. Users expect `orbit job run` to trigger execution and `orbit job runs` or `orbit job history` to inspect history. Having `JobRunCommand` at the top level is a taxonomy error that makes the CLI harder to discover.

### 3. Status value inconsistency: hyphen vs underscore

`--status in-progress` is the valid CLI value (enforced by clap), but `examples.md` uses `in_progress`, and `orbit task show` prints `in_progress` in its output. I encountered this directly during this session. The CLI, the display layer, and the documentation do not agree.

Any automation pipeline that reads output and feeds it back as input will break silently.

### 4. Activity has no update command

`orbit activity` supports add/list/show/run/delete. There is no update. Changing an activity's description, instruction, schedule-binding, or skill refs requires delete-and-recreate. That is destructive and loses the created_at timestamp. Activities are configuration—they should be updateable in place.

### 5. Audit log is a black box from the CLI

Orbit's core invariant is auditability. But `orbit audit` only exposes `prune` and `export`. There is no `orbit audit list`, `orbit audit show <id>`, or `orbit audit query --since`. Operators cannot inspect what the system did without extracting raw SQLite or exporting everything. An audit trail that can't be queried is not useful as a product feature.

### 6. Duplicate duration parsing

`parse_duration_seconds` is duplicated verbatim in `activity.rs` and `job.rs`. In a codebase where strict crate layering is an explicit invariant, this is a violation hiding in plain sight. It belongs in `orbit-exec` or a shared utility module.

### 7. `examples.md` is stale

`examples.md` references `--owner`, `--parent`, and `task close`/`task reopen` commands that do not exist in the current CLI. These examples will mislead every new operator. They should be removed or corrected.

---

## Recommendations by Priority

### Immediate (quick wins, low risk)

| # | Change | Why |
|---|--------|-----|
| 1 | Implement `orbit task list --ops` with a lean signal view | Cuts agent token usage ~10x per operational query |
| 2 | Move execution summaries to `.orbit/reports/<task>.md` with path reference in task | Removes largest single source of token bloat |
| 3 | Fix `--status` value consistency (pick one: hyphen or underscore, apply everywhere) | Breaks automation pipelines silently today |
| 4 | Add `orbit activity update` | Avoids destructive workaround for routine changes |
| 5 | Fix or remove stale commands in `examples.md` | Misleads every new user |
| 6 | Deduplicate `parse_duration_seconds` into a shared module | Enforces the crate layering invariant the codebase already declares |

### Structural (significant but contained)

| # | Change | Why |
|---|--------|-----|
| 7 | Merge `orbit job-run` into `orbit job` subcommand namespace | Fixes CLI taxonomy; `orbit job runs`, `orbit job archive-run` |
| 8 | Add `orbit audit list` / `orbit audit show` / `orbit audit query` | Makes the audit invariant observable, not just present |
| 9 | Implement three-tier output model (signal/summary/artifact) across task, job, activity | Reduces agent session costs to 800–1200 tokens from ~14k |

### Larger bets (strategic, longer horizon)

| # | Change | Why |
|---|--------|-----|
| 10 | Task dependency graph (`--depends-on`, `--blocked-by`) | Enables multi-task workflows; currently each task is an island |
| 11 | `orbit watch` — live terminal view of running jobs and tasks | Makes the runtime observable without polling; critical for operator confidence |
| 12 | `orbit replay --since <timestamp>` from audit log | Allows reconstructing system state at any point; turns the audit log into a debugging tool |
| 13 | Structured diff output in job run history | When an agent run produces code changes, expose a structured diff rather than raw logs; makes review workflows tractable |

---

## Overall Assessment

Orbit has the right architecture. The invariants are sound. The job system is clean. The skill model is original and the right level of abstraction.

The immediate problem is not structural—it is polish and operational ergonomics. The token bloat issue alone makes autonomous operation expensive and fragile. The CLI inconsistencies (status values, stale examples, missing activity update) add friction that compounds over time as more agents depend on the system.

Fix the signal/summary/artifact output model first. Everything else follows from that.

The bigger bets (dependency graph, live watch, audit replay) are the features that would make Orbit genuinely differentiated from a task runner or a cron scheduler. They are worth doing, but only after the operational baseline is solid.

---

*This report is analysis-only. Each section can be converted to a concrete Orbit task for execution.*
