# Execution Summary - Multi-Step Job Schema + Directory-Based Run Storage
Agent Name: Grace
Agent Model: claude-sonnet-4-6

## Status
success

## Orbit Task
Task ID: T20260314-220354

## 1. Summary of Changes

**Multi-step Job schema (orbit-types, orbit-store, orbit-core, orbit-cli):**
- Added `JobStep` struct: `{ target_type, target_id, agent_cli, timeout_seconds, env_extra }`
- Added `JobRunStep` struct: per-step execution record with timing, exit code, state, error fields
- `Job` struct: replaced flat step fields with `steps: Vec<JobStep>`
- `JobRun` struct: step-level fields (`exit_code`, `agent_response_json`, `error_code`, `error_message`) marked `#[serde(skip)]` and populated in-memory from step files; added `steps: Vec<JobRunStep>` also `#[serde(skip)]`
- `JobRunCompletionParams` replaced by `JobRunStepParams`; store trait split into `complete_job_run_step` + `finalize_job_run`
- `orbit-core` job execution loops over steps; stale run recovery writes synthetic step file
- `orbit-cli` displays multi-step jobs; backward-compat flat fields preserved in JSON output from `steps.first()`
- All 5 YAML job asset files updated to multi-step schema

**Directory-based run storage (orbit-store):**
- Run bundle: `runs/<job_id>/<run_id>/jrun.yaml`
- Step files: `runs/<job_id>/<run_id>/steps/<01>-<target_id>.yaml`
- `read_run_at(dir)` parses both jrun.yaml and step files to populate in-memory fields
- Archive/delete operate on directories (`fs::rename`, `fs::remove_dir_all`)
- All path helpers updated; new `run_bundle_dir()` / `archived_run_bundle_dir()`

## 2. Strategic Decisions

- **`#[serde(skip)]` on backward-compat fields** | Rationale: Avoids jrun.yaml format change and widespread CLI/test breakage; step-level data lives in step files | Trade-offs: In-memory population adds read complexity; serialized `JobRun` no longer contains error info directly
- **Flat fields preserved in `job_to_json` CLI output** | Rationale: Single-step jobs are the common case; downstream tooling expects `target_type` etc. at top level | Trade-offs: Two sources of truth (flat + `steps` array) in JSON response
- **Index-prefixed step filenames (`01-<target_id>.yaml`)** | Rationale: Deterministic ordering; avoids collision when same target appears in multiple steps | Trade-offs: Max 99 steps without zero-padding adjustment (acceptable)
- **Split `complete_job_run_step` / `finalize_job_run`** | Rationale: Clean separation between step completion and run finalization; enables partial step data on stale recovery | Trade-offs: Callers must call both; slightly more API surface

## 3. Assumptions Made

- Single-step jobs remain the overwhelmingly common case | Impact if incorrect: `steps.first()` backward-compat shim in CLI would be confusing if multi-step is the norm
- Step index is zero-based in `JobRunStep` but one-based in filenames (e.g. `01-`) | Impact if incorrect: Off-by-one in filename parsing

## 4. Design Weaknesses / Risks

- **Dual representation of step fields on `JobRun`** | Severity: Medium | Mitigation: Long-term, remove flat compat fields once all consumers use `steps` array
- **No migration for existing flat jrun-*.yaml files** | Severity: Low (dev-only data root) | Mitigation: Data root is ephemeral in development; production rollout would need a migration pass
- **Step file parsing skips unrecognized files silently** | Severity: Low | Mitigation: Log a warning on unexpected filenames in step dir

## 5. Deviations from Original Plan

- `JobRunStep.state` field added beyond original spec | Justification: Needed for per-step success/failure recording without relying on `exit_code` alone
- `JobRunStepFileDocument` wrapper struct added | Justification: Consistent with `JobRunFileDocument` pattern for YAML serialization with schema version

## 6. Technical Debt Introduced

- Flat backward-compat fields (`target_type`, `target_id`, etc.) remain in `job_to_json` CLI output | Recommended resolution: Deprecate in next minor, remove once consumers adopt `steps`
- `#[serde(skip)]` on `JobRun.exit_code` etc. means serialization round-trips lose these fields | Recommended resolution: Remove flat fields from `JobRun` entirely once CLI layer uses `steps[0]` directly

## 7. Recommended Follow-Ups

- Add structured step display to `orbit job show` (currently only step count shown)
- Add `orbit job history --run-id <id> steps` subcommand to inspect per-step detail
- Add migration utility to convert existing flat `jrun-<id>.yaml` to directory bundles
- Consider removing `#[serde(skip)]` compat fields from `JobRun` in a follow-up cleanup

## 8. Overall Assessment

Implementation is complete and all 237 tests pass. The schema change is backward-compatible at the CLI layer and the new directory-based run storage is well-tested with a dedicated round-trip test. The main technical debt is the dual representation of step-level fields on `JobRun`, which is intentional for now but should be cleaned up once consumers are updated.
