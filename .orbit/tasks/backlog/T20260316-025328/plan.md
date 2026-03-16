# Task Bundle Creation Hardening Plan

**Goal:** Make task creation resilient against partial or malformed bundle writes and ensure Orbit never reports success for an unreadable task bundle.
**Scope:** file-backed task persistence, task creation/update verification, rollback behavior, and regression coverage for malformed bundle scenarios.
**Assumptions:** The observed zero-byte `task.yaml` likely came from an interrupted or externally corrupted write path, but Orbit should still defend against that class of failure.
**Risks:** Touching persistence requires care to avoid regressions in normal task lifecycle operations.

## Task 1: Audit bundle-write atomicity and durability

**Files:**
- Modify: `orbit-store/src/file/task_store.rs`
- Read/Modify: any shared file-write helpers used by file-backed stores

**Steps:**
1. Inspect how task bundles are written today and identify where partial bundle state can leak onto disk.
2. Decide whether task bundle creation should use a staging directory plus atomic directory rename, stronger fsync semantics, or equivalent validation/rollback protections.
3. Document the chosen write contract in code comments or tests.

**Done When:**
- There is one clear, intentional bundle-write strategy rather than best-effort per-file writes.

## Task 2: Fail closed on malformed bundle creation

**Files:**
- Modify: `orbit-store/src/file/task_store.rs`
- Modify: `orbit-core/src/command/task.rs` if needed for improved error handling
- Modify: `orbit-cli/src/command/task.rs` only if user-facing error/reporting needs adjustment

**Steps:**
1. Ensure task creation either produces a fully readable bundle or returns an error.
2. Re-read/validate the created bundle before success is returned, or otherwise guarantee equivalent integrity.
3. Add rollback or cleanup for incomplete bundles so future reads do not get poisoned by malformed task directories.

**Done When:**
- `orbit task add` cannot leave behind an unreadable task bundle while still reporting success.

## Task 3: Add regression coverage for malformed bundle scenarios

**Files:**
- Modify: `orbit-store/src/file/task_store.rs` tests
- Modify: `orbit-cli/tests/task_commands.rs` if CLI-level coverage is useful

**Steps:**
1. Add tests for malformed/partial bundle detection during reads.
2. Add tests that prove create/update paths do not leave corrupted task bundles behind on failure.
3. If practical, simulate interrupted-write-like conditions or a failed sidecar write.

**Done When:**
- The observed failure mode is covered by regression tests or the closest feasible simulation.

## Final Verification
- `cargo test -p orbit-store`
- `cargo test -p orbit-cli --test task_commands`
- `cargo test --workspace`