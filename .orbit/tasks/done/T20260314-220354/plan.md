# Multi-step Job Schema + Directory Job-Run Refactor

**Goal:** Replace flat single-step Job schema with a steps array, and replace flat jrun YAML with a directory bundle.
**Scope:** orbit-types, orbit-store, orbit-core (types, store, seeding, CLI display). No behavioral change to job scheduling logic.
**Assumptions:** All existing job runs on disk are treated as legacy; no migration of existing run files required (they will not be readable by new code unless a compat reader is added — acceptable since runs are ephemeral operational data).
**Risks:** `complete_job_run` currently writes agent_response_json at the top level — the new API needs to target a specific step. Store interface changes ripple into orbit-core and CLI callers.

## Sub-task 1: Update type definitions in orbit-types

**Files:**
- Modify: `orbit-types/src/job.rs`

**Steps:**
1. Add `JobStep` struct with fields: `target_type: JobTargetType`, `target_id: OrbitId`, `agent_cli: String`, `timeout_seconds: u64`, `env_extra: Vec<String>`
2. Replace flat fields on `Job` (`target_type`, `target_id`, `agent_cli`, `timeout_seconds`, `env_extra`) with `steps: Vec<JobStep>`
3. Add `JobRunStep` struct with fields: `target_type: JobTargetType`, `target_id: OrbitId`, `started_at: Option<DateTime<Utc>>`, `finished_at: Option<DateTime<Utc>>`, `duration_ms: Option<u64>`, `exit_code: Option<i32>`, `agent_response_json: Option<Value>`, `status: JobRunState`, `error_code: Option<String>`, `error_message: Option<String>`
4. Remove step-level fields from `JobRun` (`exit_code`, `agent_response_json`, `error_code`, `error_message`); keep run-level fields only
5. Write unit tests verifying round-trip serde for `JobStep` and `JobRunStep`
6. Run: `cargo test -p orbit-types`

**Done When:**
- orbit-types compiles and tests pass
- `Job` has `steps: Vec<JobStep>`, no flat activity fields
- `JobRun` has no step-level fields

## Sub-task 2: Update job YAML asset files and seeding code

**Files:**
- Modify: `orbit-core/assets/jobs/job-approve-task-leader.yaml` (and 4 others)
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Update all 5 YAML asset files to use `steps:` array (each has exactly one step)
2. Update `DefaultJobEntry` deserialization struct to have `steps: Vec<DefaultJobStep>` instead of flat fields
3. Add `DefaultJobStep` struct matching the step YAML shape
4. Update `seed_default_jobs()` mapping to populate `Job.steps` from `DefaultJobEntry.steps`
5. Run: `cargo test -p orbit-core -- seed`

**Done When:**
- All 5 asset files use steps array
- Seeding code parses and maps steps correctly
- Bundled parse tests pass

## Sub-task 3: Update store to directory-based job-run storage

**Files:**
- Modify: `orbit-store/src/file/job_store.rs`
- Modify: `orbit-store/src/backend/contracts.rs` (`JobRunCompletionParams`)

**Steps:**
1. Write failing tests for: write a run directory + step file, read it back, archive it (move directory)
2. Run failing tests: `cargo test -p orbit-store -- job_run`
3. Add `JobRunStepFileDocument` wrapper struct for step YAML (`schemaVersion + step:`)
4. Update `write_run()` to:
   - Write `<run_dir>/jrun.yaml` with run-level fields only
   - Write `<run_dir>/steps/<index>-<target_id>.yaml` for each step
5. Update `read_run_at()` to accept a run directory path, read `jrun.yaml` + all step files
6. Update `insert_job_run()` to create directory and write initial jrun.yaml + empty step placeholders
7. Update `mark_job_run_running()` to update jrun.yaml state
8. Split `complete_job_run()` into:
   - `complete_job_run_step(run_id, step_index, params)` — writes final step file
   - `complete_job_run(run_id, state)` — finalizes jrun.yaml
9. Update `archive_run()` to move the entire run directory (not just a file)
10. Update `read_runs_for_activity()` and `read_all_runs()` to scan directories
11. Update `JobRunCompletionParams` to include `step_index: usize`
12. Re-run tests: `cargo test -p orbit-store`

**Done When:**
- All store tests pass
- A run produces `jrun.yaml` + `steps/01-<id>.yaml` on disk
- Archive moves the directory correctly

## Sub-task 4: Update orbit-core callers and CLI display

**Files:**
- Modify: `orbit-core/src/lib.rs` or wherever `complete_job_run` is called
- Modify: any CLI command that displays job-run fields (`exit_code`, `agent_response_json`, etc.)

**Steps:**
1. Find all callers of `complete_job_run` in orbit-core: `grep -r complete_job_run orbit-core/`
2. Update each caller to use `complete_job_run_step` + `complete_job_run`
3. Update any display code that accesses top-level `JobRun.exit_code` etc. to use `run.steps[n].exit_code`
4. Run: `cargo build --workspace`
5. Run: `cargo test --workspace`

**Done When:**
- Workspace builds without errors
- All tests pass
- `orbit job run list` (or equivalent) shows correct output

## Final Verification
```bash
cargo build --workspace
cargo test --workspace
orbit job seed           # seeds default jobs with new schema
orbit job list           # jobs show steps field
```