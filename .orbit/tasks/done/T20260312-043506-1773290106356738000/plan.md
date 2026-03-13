# Warning Cleanup Plan

**Goal:** Eliminate the unused-method warning for `ExecutionEnvPolicy::hydrated_allowlist_env` without changing intended runtime behavior.
**Scope:** `orbit-core` execution-env configuration and the call sites/tests that cover allowlisted environment hydration.
**Assumptions:** The no-argument helper is redundant unless another runtime path still needs a convenience wrapper.
**Risks:** Removing the method could break a future internal caller if there is undocumented intended usage; keeping it without a real call site preserves warning noise.

## Task 1: Confirm intended API surface

**Files:**
- Modify: `orbit-core/src/config/runtime.rs`
- Review: `orbit-core/src/command/job.rs`
- Review: `orbit-core/src/runtime/mod.rs`

**Steps:**
1. Inspect `ExecutionEnvPolicy` callers and decide whether the no-argument helper should exist.
2. Remove the unused wrapper or add a legitimate internal caller only if it improves clarity.
3. Keep the allowlist hydration behavior unchanged for job execution paths.

**Done When:**
- `orbit-core` no longer emits the `hydrated_allowlist_env` dead-code warning.
- Execution environment hydration behavior remains equivalent for current job flows.

## Task 2: Protect behavior with focused verification

**Files:**
- Test: `orbit-core/src/config/mod.rs`
- Test: `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Add or adjust targeted tests if the refactor changes exposed internal behavior.
2. Run focused `orbit-core` tests covering execution environment policy and job runtime handling.
3. Run a package-level build/test pass to confirm the warning is gone.

**Done When:**
- Relevant config/runtime tests pass.
- A package-level `orbit-core` test or build finishes without this warning.

## Final Verification
- `cargo test -p orbit-core`
- `cargo build -p orbit-core`