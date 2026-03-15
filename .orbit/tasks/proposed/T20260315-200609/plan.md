# dispatch_task Guard and Rejected-Task Priority Plan

**Goal:** Skip agent invocation when queues are empty; surface rejected tasks with higher priority than backlog.
**Scope:** orbit-types JobStep struct, orbit-core job runner, dispatch_task activity YAML (bundled + active).
**Assumptions:** A clean "nothing to do" stop is not a job failure — it should complete with a descriptive comment.
**Risks:** Precondition check adds a new exit-code contract between CLI and job runner; must be explicit.

## Task 1: Add `precondition` to JobStep in orbit-types

**Files:**
- Modify: `orbit-types/src/job.rs` — `JobStep` struct

**Steps:**
1. Add field to `JobStep`:
   ```rust
   #[serde(skip_serializing_if = "Option::is_none")]
   pub precondition: Option<JobStepPrecondition>,
   ```
2. Define new struct:
   ```rust
   pub struct JobStepPrecondition {
       pub command: String,         // shell command to run
       pub args: Vec<String>,
       pub skip_job_on_failure: bool, // if true, non-zero exit stops job cleanly (not as error)
   }
   ```
3. Write unit test: precondition serialises/deserialises correctly from YAML.

**Done When:**
- `cargo test -p orbit-types` passes.

## Task 2: Enforce precondition in job runner

**Files:**
- Modify: `orbit-core/src/command/job.rs` — step execution loop

**Steps:**
1. Before executing each step, check if `step.precondition` is set.
2. Run the precondition command via `std::process::Command`.
3. If exit code is non-zero AND `skip_job_on_failure: true`:
   - Set job run state to `success` (not failed).
   - Record a comment like "Precondition not met for step '<step_id>': skipped cleanly."
   - Stop the pipeline without error.
4. If exit code is non-zero AND `skip_job_on_failure: false`:
   - Treat as step failure (existing error path).
5. Add regression test: job with a failing precondition + skip_job_on_failure:true completes as success with correct comment.

**Done When:**
- New regression test passes.
- `cargo test --workspace` green.

## Task 3: Add `check_dispatch_needed` activity

**Files:**
- Create: `orbit-core/assets/activities/check_dispatch_needed.yaml`

**Content:**
```yaml
activity:
  id: check_dispatch_needed
  spec_type: cli_command
  description: Exits 0 if backlog or rejected tasks exist; exits 1 if both queues are empty.
  command: bash
  args:
    - -c
    - |
      COUNT=$(orbit task list --status backlog --json 2>/dev/null | jq 'length')
      REJECTED=$(orbit task list --status rejected --json 2>/dev/null | jq 'length')
      if [ "$COUNT" -eq 0 ] && [ "$REJECTED" -eq 0 ]; then exit 1; fi
  expected_exit_codes: [0]
```

**Done When:**
- Activity file exists and is valid YAML.
- Manual test: `orbit activity run check_dispatch_needed` exits 0 when tasks exist, 1 when empty.

## Task 4: Update dispatch_task instruction

**Files:**
- Modify: `orbit-core/assets/activities/dispatch_task.yaml`
- Modify: `.orbit/activities/active/dispatch_task.yaml`

**Changes to instruction:**
1. Step 1: query both `status: "backlog"` AND `status: "rejected"` via two `orbit.task.list` calls.
2. If both lists are empty, return `task_id: "None"` and exit (defensive fallback, though precondition should prevent this).
3. Prioritise rejected tasks first: "A rejected task represents work already reviewed and found incomplete — it takes precedence over unstarted backlog items."
4. Within each group, apply existing priority rules (critical > high > medium > low).

**Done When:**
- Both YAML files updated consistently.
- Instruction clearly explains rejected > backlog priority ordering.

## Task 5: Wire precondition into job_task_pipeline

**Files:**
- Modify: `.orbit/jobs/jobs/job_task_pipeline.yaml`
- Modify: `orbit-core/assets/jobs/job_task_pipeline.yaml` (if bundled)

**Changes:**
Add precondition to the dispatch_task step:
```yaml
- target_type: activity
  target_id: dispatch_task
  agent_cli: claude
  timeout_seconds: 1000
  env_extra: []
  precondition:
    command: orbit
    args: [activity, run, check_dispatch_needed]
    skip_job_on_failure: true
```

**Done When:**
- Job YAML is valid.
- End-to-end: running job_task_pipeline with empty queues stops cleanly with success state.

## Final Verification
- `cargo test --workspace` — all green
- Job run with empty queues: state = success, comment mentions precondition skip
- Job run with backlog tasks: agent invoked, task dispatched normally
- Job run with only rejected tasks: agent invoked, rejected task selected with higher priority