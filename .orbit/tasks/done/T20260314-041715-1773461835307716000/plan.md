# Remove Job Scheduler — Implementation Plan

**Goal:** Delete all scheduling machinery; jobs run only on explicit orbit job run.
**Scope:** Types, store backends, job runtime, CLI commands, tests.
**Assumptions:** Retries become synchronous (sleep-then-retry within same invocation). Confirm before starting if the proposer wants a different retry model.
**Risks:** orbit job serve and orbit job tick are referenced in the CLAUDE.md Job System section — update docs too.

---

## Task 1: Simplify Job types in orbit-types

**Files:**
- Modify: orbit-types/src/job.rs

**Steps:**
1. Write a failing test that constructs a Job without schedule/next_run_at fields.
   Run: cargo test -p orbit-types -- --test-threads=1
2. Remove fields from Job struct: schedule, next_run_at.
3. Remove JobScheduleState::Paused variant; rename enum to JobState or keep as-is with only Enabled/Disabled.
4. Remove any schedule-related helper constructors (e.g., manual_job_next_run_at()).
5. Fix all compilation errors introduced (orbit-types has no deps on other workspace crates, so errors are contained).
6. Re-run: cargo test -p orbit-types -- --test-threads=1

**Done When:**
- Job struct has no schedule or next_run_at fields.
- JobScheduleState (or JobState) has only Enabled and Disabled variants.
- cargo test -p orbit-types passes.

---

## Task 2: Delete orbit-core/src/job/state_machine.rs

**Files:**
- Delete: orbit-core/src/job/state_machine.rs
- Modify: orbit-core/src/job/mod.rs (remove mod state_machine declaration)

**Steps:**
1. Confirm no callers outside job/runtime.rs and job/job.rs reference state_machine functions.
   Run: grep -r 'state_machine\|compute_next_run_at\|compute_next_cron\|parse_interval' orbit-core/src/
2. Delete orbit-core/src/job/state_machine.rs.
3. Remove the module declaration from orbit-core/src/job/mod.rs.
4. Fix any remaining compilation errors (expected: callers in command/job.rs use compute_next_run_at).
   Run: cargo check -p orbit-core 2>&1 | head -40

**Done When:**
- state_machine.rs is deleted.
- cargo check -p orbit-core compiles (other tasks may introduce new errors; that is expected).

---

## Task 3: Remove due-claim machinery from job store

**Files:**
- Modify: orbit-store/src/backend/contracts.rs
- Modify: orbit-store/src/file/job_store.rs
- Modify: orbit-store/src/sqlite/job_store.rs
- Modify: orbit-store/src/sqlite/migration.rs (if next_run_at column referenced)

**Steps:**
1. Remove from JobStoreBackend trait: due_jobs(), next_due_job_time(), claim_due_jobs(), update_job_next_run().
2. Remove implementations of those methods from file and sqlite backends.
3. Remove next_run_at column references from sqlite migration/schema if present.
4. Fix compilation errors.
   Run: cargo check -p orbit-store 2>&1

**Done When:**
- JobStoreBackend trait has no scheduling-related methods.
- cargo check -p orbit-store compiles.

---

## Task 4: Remove job runtime scheduler (orbit-core/src/job/runtime.rs and job.rs)

**Files:**
- Modify: orbit-core/src/job/runtime.rs
- Modify: orbit-core/src/job/job.rs

**Steps:**
1. Remove JobRuntime::tick_once(), run_forever(), run_due_jobs() from runtime.rs.
2. Remove JobTickResult type.
3. Remove run_due_jobs(), claim_due_jobs() and related logic from job.rs.
4. If job/runtime.rs is now empty, delete it and remove from mod.rs.
5. Fix compilation errors.
   Run: cargo check -p orbit-core 2>&1

**Done When:**
- No tick/serve/due-claim logic remains in orbit-core/src/job/.
- cargo check -p orbit-core compiles (CLI errors expected until Task 5).

---

## Task 5: Remove serve, tick, pause, resume from command layer

**Files:**
- Modify: orbit-core/src/command/job.rs
- Modify: orbit-cli/src/command/job.rs

**Steps:**
1. In orbit-core/src/command/job.rs:
   - Remove JobServeCommand, JobTickCommand, JobPauseCommand, JobResumeCommand and their execute() impls.
   - Remove --schedule arg from JobAddCommand; remove next_run_at initialization.
   - Remove post-execution compute_next_run_at call; after a job run completes, no schedule update needed.
   - For retries: rewrite compute_retry_delay_seconds logic to sleep-then-retry synchronously instead of scheduling next_run_at.
2. In orbit-cli/src/command/job.rs:
   - Remove JobServeArgs, JobTickArgs, serve/tick/pause/resume subcommand variants.
   - Remove --schedule flag from JobAddArgs.
   - Update JobSubcommand enum and match arms.
3. Fix remaining compilation errors.
   Run: cargo check --workspace 2>&1

**Done When:**
- orbit job serve, tick, pause, resume no longer exist as commands.
- orbit job add has no --schedule flag.
- cargo check --workspace compiles.

---

## Task 6: Update YAML job files and seeds

**Files:**
- Modify: orbit-core/src/command/job.rs (seed job definitions)
- Modify: .orbit/jobs/jobs/*.yaml (existing job files on disk)

**Steps:**
1. Remove schedule and next_run_at fields from all seed job structs/literals in code.
2. For each .yaml file in .orbit/jobs/jobs/: remove schedule and next_run_at keys.
3. Verify job files still deserialize correctly.
   Run: cargo test -p orbit-core -- --test-threads=1

**Done When:**
- No job YAML file or seed definition contains schedule or next_run_at.

---

## Task 7: Update tests

**Files:**
- Modify: orbit-core/tests/job_runtime_behavior.rs
- Modify: orbit-cli/tests/job_commands.rs
- Modify: orbit-store/tests/job_store_integration.rs

**Steps:**
1. Delete tests for tick, serve, pause, resume, due-claim, and cron parsing behavior.
2. Rewrite any test using --schedule flag to omit it.
3. Rewrite any test asserting next_run_at or schedule field values.
4. Run: make test

**Done When:**
- make test passes with zero failures.
- No test references schedule, next_run_at, tick, serve, due_jobs, or claim_due_jobs.

---

## Task 8: Update CLAUDE.md

**Files:**
- Modify: CLAUDE.md

**Steps:**
1. In the Job System section, remove orbit job serve and orbit job tick entries.
2. Remove 'Jobs use due-claim semantics with overlap prevention' sentence.
3. Update to: 'Jobs are manually triggered via orbit job run <id>'.

**Done When:**
- CLAUDE.md Job System section is accurate.

---

## Final Verification

```bash
make fmt
make clippy
make test
grep -r 'schedule\|next_run_at\|tick_once\|run_forever\|due_jobs\|claim_due' orbit-core/src/ orbit-cli/src/ orbit-store/src/ --include='*.rs' | grep -v test | grep -v target
```

No non-test source should reference scheduler concepts after these changes.