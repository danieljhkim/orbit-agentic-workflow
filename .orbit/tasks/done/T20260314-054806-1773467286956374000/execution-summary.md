# Execution Summary - orbit job list should show last-run status inline
Agent Name: Grace
Agent Model: claude-sonnet-4-6

## Status
success

## Orbit Task
Task ID: T20260314-054806-1773467286956374000

## 1. Summary of Changes
- Added `list_jobs_with_last_run` public method to `OrbitRuntime` in `orbit-core/src/command/job.rs`. Returns `Vec<(Job, Option<JobRun>)>` with stale-run recovery per job before reading the last run.
- Updated `JobListArgs::execute` in `orbit-cli/src/command/job.rs` to call the new method and display a `LAST_RUN` column in table output (showing "never" or "{state} {timestamp}").
- Updated JSON output for `orbit job list --json` to include `last_run_state` and `last_run_at` fields.
- The `--ops` signal-tier JSON output is unchanged (minimal fields only).
- Added integration test `job_list_shows_last_run_status_in_table_and_json` covering the no-run/never case, post-run success case, and JSON field presence.

## 2. Strategic Decisions
- N+1 store queries per job | Rationale: Each job needs its own last run; the file-based store's `list_job_runs_filtered` is a cheap local directory scan | Trade-offs: Could batch with a full run scan + group-by, but that loads all run history and is slower for few jobs.
- Stale recovery in list path | Rationale: Consistent with `job_history` behavior; ensures displayed state is accurate | Trade-offs: Adds minor overhead to `job list`.
- Timestamp format `%Y-%m-%dT%H:%M:%SZ` in table | Rationale: Compact, human-readable, avoids sub-second noise | Trade-offs: Not strict RFC 3339 (no fractional seconds), but readable.
- `finished_at` preferred over `started_at` for terminal states | Rationale: Most meaningful for health monitoring | Trade-offs: For pending/running runs, falls back to started_at then scheduled_at.

## 3. Assumptions Made
- `--ops` consumers do not need last-run data | Impact if incorrect: Would need a separate field in signal-tier JSON.

## 4. Design Weaknesses / Risks
- N+1 queries | Severity: Low | Mitigation: Acceptable at Orbit's operational scale; can optimize later with a batch store method if needed.

## 5. Deviations from Original Plan
None.

## 6. Technical Debt Introduced
None.

## 7. Recommended Follow-Ups
- Consider adding `last_run_state` to `--ops` JSON if signal-tier consumers need health at a glance.

## 8. Overall Assessment
Clean, minimal implementation that satisfies the feature request with full test coverage and no regressions.