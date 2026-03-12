# Fix: Guard manual schedule in post-execution next-run computation

**Goal:** Prevent `compute_next_run_at` from being called with `"manual"` after job execution.
**Scope:** `orbit-core/src/command/job.rs` only. No schema or store changes needed.

## Step 1: Write a failing test

**File:** `orbit-core/tests/job_runtime_behavior.rs`

Add a test: add a manual-schedule job, call `run_job_now`, assert result is `Success` (not an error). This test must fail before the fix.

Run: `cargo test -p orbit-core run_manual_schedule_job_does_not_error_on_cron_validation`

## Step 2: Apply the fix

**File:** `orbit-core/src/command/job.rs`, lines 369–376

Replace the unconditional `compute_next_run_at` call with the same guard used in `add_job` and `resume_job`:

```rust
if !retry_scheduled_for_future {
    let next_run_at = if job.schedule.trim().eq_ignore_ascii_case("manual") {
        manual_job_next_run_at()
    } else {
        crate::job::state_machine::compute_next_run_at(&job.schedule, Utc::now())?
    };
    let _ = self.update_job_next_run_backend(&job.job_id, next_run_at);
    let _ = self.record_event(OrbitEvent::JobTriggered { job_id: job.job_id.clone() });
}
```

Run: `cargo test -p orbit-core run_manual_schedule_job_does_not_error_on_cron_validation`

## Step 3: Full verification

```bash
make ci
cargo test -p orbit-core
cargo test -p orbit-cli job
```

**Done When:**
- `orbit job run job-resolve-backlogged-task` completes without cron validation error.
- All tests pass under `make ci`.