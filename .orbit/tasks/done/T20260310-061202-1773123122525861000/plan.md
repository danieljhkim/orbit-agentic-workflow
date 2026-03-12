# Implementation Plan

**Goal:** Ship three job system capabilities: `--task-id` on job run, `--job-id` on job add, and `--schedule manual` support.
**Scope:** `orbit-cli`, `orbit-core`, `orbit-store`. No changes to activity or task domain.
**Assumptions:** `task_id` injection into agent context already works via `run_job_now_with_input`; only the CLI flag exposure needs verification.
**Risks:** Schedule parser change must not affect existing cron/every-N jobs. Named ID insert must guard against duplicates.

## Task 1: Verify and expose orbit job run --task-id

**Files:**
- Read: `orbit-cli/src/command/job.rs` lines 168–175 (`JobRunArgs`)
- Read: `orbit-core/src/command/job.rs` (`run_job_now_with_input` impl)

**Steps:**
1. Confirm `task_id` field has `#[arg(long)]` annotation; add it if missing.
2. Add failing test: `orbit job run <job_id> --task-id T123` passes `{"task_id":"T123"}` as run input.
3. Run `cargo test -p orbit-cli job_run_task_id`.
4. Run `make build` and verify `orbit job run --help` shows `--task-id`.
5. Run `make ci`.

**Done When:**
- `orbit job run --help` lists `--task-id`
- Run input contains the injected task ID

## Task 2: Add --job-id to orbit job add

**Files:**
- Modify: `orbit-core/src/command/job.rs` — add `job_id: Option<String>` to `JobAddParams`
- Modify: `orbit-store/src/file/job_store.rs` — use provided ID or fall back to `next_id`; reject if already exists
- Modify: `orbit-cli/src/command/job.rs` — add `#[arg(long)] pub job_id: Option<String>` to `JobAddArgs`
- Test: add integration test

**Steps:**
1. Add failing test: `orbit job add --job-id job-foo ...` creates job with id `job-foo`.
2. Thread `job_id` from CLI → `JobAddParams` → store insert.
3. In store: if `job_id` is `Some`, use it; else call `next_id`. Return error if ID already exists.
4. Run `cargo test -p orbit-cli job_add_named_id`. Run `make ci`.

**Done When:**
- `orbit job add --job-id job-foo ...` creates job with exactly id `job-foo`
- Duplicate ID returns non-zero error
- Auto-generated ID behavior unchanged when flag is omitted

## Task 3: Add --schedule manual support

**Files:**
- Read first: `orbit-core/src/job/` (scheduling, due-claim cycle)
- Modify: schedule validation to accept `manual` as a valid value
- Modify: job insert to set state `disabled` and skip `next_run_at` when schedule is `manual`
- Modify: due-claim tick to skip `manual`-schedule jobs
- Test: add failing test

**Steps:**
1. Add failing test: job created with `--schedule manual` has state `disabled` and is never returned by `orbit job tick`.
2. Accept `manual` string in schedule validation; set `next_run_at` to far future or epoch.
3. Ensure due-claim loop skips jobs where schedule is `manual` or state is `disabled`.
4. Confirm `orbit job run <id>` still works (explicit trigger bypasses schedule check).
5. Run `make ci`.

**Done When:**
- `orbit job add --schedule manual ...` succeeds, job state is `disabled`
- `orbit job tick` never claims a manual job
- `orbit job run <id>` triggers the job on demand regardless

## Final Verification
```bash
make build
orbit job run --help                          # --task-id must appear
make ci
orbit job add --job-id job-smoke-test --target-id resolve-backlogged-task --schedule manual --agent-cli claude --timeout 5m
orbit job show job-smoke-test                 # state: disabled, schedule: manual
orbit job tick                                # smoke-test job must NOT appear
orbit job run job-smoke-test --task-id T999   # must trigger without error
orbit job delete job-smoke-test
```