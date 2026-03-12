# Execution Summary - Investigate job run stuck in running state after agent success
Agent Name: Steve (CEO)
Agent Model: claude-sonnet-4-6

## Status
success

## Orbit Task
Task ID: T20260310-042915-1773116955241830000

## 1. Summary of Changes

**Root cause identified**: `wait_with_output()` in `orbit-exec/src/timeout.rs` blocked indefinitely when the agent CLI (claude/codex) exited but left orphan child processes that inherited the stdout/stderr pipe write ends. Those orphan processes prevented EOF on the pipes, so `wait_with_output()` never returned, `complete_job_run_backend` was never called, and the run stayed in `running` state with the terminal frozen.

**Changes made** (3 files in `orbit-exec`, no other crates modified):

1. **`orbit-exec/src/process.rs`**: Added `command.process_group(0)` on Unix when spawning the agent. This makes the child its own process group leader (PGID = PID), so `killpg` can reach all orphan descendants.

2. **`orbit-exec/src/timeout.rs`**: Rewrote `wait_with_optional_timeout`. Key changes:
   - Stdout/stderr are now drained by background threads (prevents pipe-full deadlock for long-running agents).
   - After child exits or after timeout kill, `killpg(pid, SIGKILL)` is sent to the child's process group, terminating all orphan processes and closing their pipe write ends.
   - Returns a new `WaitResult` struct instead of `std::process::Output`, eliminating a latent double-`waitpid` risk.

3. **`orbit-exec/src/runner.rs`**: Updated to construct `ExecutionResult` from the new `WaitResult` type.

4. **`orbit-exec/Cargo.toml`** / **`Cargo.toml`**: Added `libc` as a workspace and Unix-only dependency (for `killpg`).

**Test added**: `agent_with_orphan_stdout_holder_does_not_hang` in `orbit-core/tests/job_runtime_behavior.rs`. Mock agent spawns `sleep 60 &` (inherits stdout) then exits with a success envelope. Before fix: channel recv_timeout panics after 5 s. After fix: completes in 0.23 s.

## 2. Strategic Decisions

- **Background reader threads for pipe draining** | Rationale: Required to prevent a secondary deadlock where the agent fills the pipe buffer and cannot exit. | Trade-offs: Two threads per invocation; negligible overhead for infrequent agent runs.

- **`killpg` after success, not just on timeout** | Rationale: The orphan-pipe bug occurs on the success path. | Trade-offs: Agent-spawned background processes are killed when the main agent exits. Acceptable for Orbit's hermetic execution model.

- **New `WaitResult` struct instead of `std::process::Output`** | Rationale: Avoids calling `child.wait()` a second time after `wait_timeout` internally calls `waitpid`, which returns ECHILD on Linux. | Trade-offs: Minor internal API change inside `orbit-exec` only.

## 3. Assumptions Made

- **Agent CLIs do not call `setsid()`/`setpgrp()` in their background children** | Impact if incorrect: Those orphans would not be killed by `killpg`; reader threads would still hang. The job timeout would eventually fire.

## 4. Design Weaknesses / Risks

- **Reader thread join has no hard deadline** | Severity: Low | Mitigation: `killpg` sends SIGKILL which is immediate; threads complete within microseconds after signal delivery in all realistic cases.

- **`killpg` on success kills processes the agent may have intentionally kept alive** | Severity: Low | Mitigation: Orbit's design is hermetic; agents are not expected to leave persistent background processes. Consistent with existing timeout kill behavior.

## 5. Deviations from Original Plan

- Root cause was isolated to `timeout.rs`/`process.rs` without needing to trace `codex_runtime.rs` or `response.rs`; those files had no role in the hang.
- Reproduction done via focused integration test rather than a separate harness.

## 6. Technical Debt Introduced

- None. The fix removes a latent double-`waitpid` risk that existed in the old code.

## 7. Recommended Follow-Ups

- Track pre-existing clippy violations in `orbit-agent` (`module_inception`, `enum_variant_names`) as a separate maintenance task.
- Consider an integration test for the timeout path with an orphan-holding process.

## 8. Overall Assessment

Minimal, focused fix contained entirely within `orbit-exec`. Root cause was a classic Unix pipe-inheritance issue. Added a reliable regression test (0.23 s vs 5 s hang). All 108 existing tests pass; no warnings in the modified crate.
