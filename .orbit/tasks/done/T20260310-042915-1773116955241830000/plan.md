# Job Run Completion Investigation Plan

**Goal:** determine why Orbit does not transition a job-run to a terminal state after Codex appears to finish and emits a valid final response.
**Scope:** runtime execution flow, agent-provider invocation, process completion handling, and job-run state persistence for Codex-backed jobs.
**Assumptions:** the supplied log is accurate and the issue occurs after `task_complete`, not before the final assistant payload is produced.
**Risks:** the bug may involve a race between stdout parsing, process exit detection, and run-state mutation; a naive fix could regress timeout or protocol-violation handling.

## Task 1: Reproduce the stuck-running behavior

**Files:**
- Inspect: `orbit-core/src/command/job.rs`
- Inspect: `orbit-agent/src/providers/codex/codex_cli.rs`
- Inspect: `orbit-exec/src/runner.rs`

**Steps:**
1. Capture or add a deterministic reproduction for a Codex-backed job that emits a final success envelope but leaves the run active.
2. Verify whether the child process exits, whether stdout parsing completes, and whether `complete_job_run` is reached.
3. Add a failing regression test at the smallest reliable layer.

**Done When:**
- there is a repeatable failing test or harness that demonstrates the running-state bug.

## Task 2: Trace the terminal-state transition

**Files:**
- Inspect: `orbit-core/src/command/job.rs`
- Inspect: `orbit-agent/src/types/response.rs`
- Inspect: `orbit-agent/src/providers/codex/codex_runtime.rs`
- Inspect: `orbit-exec/src/process.rs`

**Steps:**
1. Trace the path from spawned Codex process output to `parse_and_validate_response`.
2. Confirm how Orbit decides the process is complete and how it distinguishes trailing log events from the final JSON payload.
3. Identify whether the persistence bug is caused by process-wait behavior, parsing, or state-mutation ordering.

**Done When:**
- the precise failure point is documented and linked to the reproduction.

## Task 3: Implement and verify a fix

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Modify: `orbit-agent/src/providers/codex/codex_cli.rs`
- Modify: `orbit-agent/src/types/response.rs`
- Modify: related tests

**Steps:**
1. Implement the minimal fix that ensures terminal runs are persisted once the agent has actually completed.
2. Re-run the new regression coverage and the relevant job runtime / CLI suites.
3. Confirm that success, failure, timeout, and protocol-violation paths still transition runs out of `running`.

**Done When:**
- a completed agent run no longer leaves the job-run in `running` state
- the terminal returns cleanly after the job finishes
- targeted and broader verification pass.

## Final Verification
- `cargo test -p orbit-core job_runtime_behavior`
- `cargo test -p orbit --test job_commands`
- any new focused reproduction or provider tests added for the fix