# Lock Store Warning Cleanup Plan

**Goal:** Remove the recurring `lock_store` dead-code warning by either using the lock backend meaningfully or removing the unused plumbing.
**Scope:** `OrbitContext`, runtime context construction, and the lock store backend surface directly related to this warning.
**Assumptions:** The current warning reflects either abandoned runtime plumbing or an unfinished locking integration, and Orbit should prefer a coherent design over keeping unused fields around.
**Risks:** Removing the field too quickly could break near-term plans for runtime locking; keeping it without real usage preserves noisy builds and unclear architecture.

## Task 1: Trace the intended lock-store ownership

**Files:**
- Modify: `orbit-core/src/context.rs`
- Read/Modify: `orbit-core/src/runtime/builder.rs`
- Read/Modify: `orbit-store/src/backend/contracts.rs`
- Read/Modify: `orbit-store/src/backend/factory.rs`

**Steps:**
1. Inspect where `LockStoreBackend` is constructed and whether any runtime path is expected to consume it.
2. Decide whether `OrbitContext` should own a lock backend at all.
3. Capture that decision in the implementation so the context shape matches real runtime needs.

**Done When:**
- There is one clear answer: either the lock store is used intentionally, or it is removed as unused plumbing.

## Task 2: Implement the minimal coherent cleanup

**Files:**
- Modify: `orbit-core/src/context.rs`
- Modify: `orbit-core/src/runtime/builder.rs`
- Modify any directly affected runtime/store files discovered in Task 1
- Add or update tests near the affected runtime code if behavior changes

**Steps:**
1. Remove the unused field and construction path if the lock store is not needed, or thread it into real behavior if it is.
2. Keep the change narrowly scoped to warning resolution and runtime clarity.
3. Add or update regression coverage for the chosen design when appropriate.

**Done When:**
- `orbit-core` no longer emits the unused `lock_store` warning in normal build/test flows.
- The resulting code is clearer about how locking is meant to work.

## Final Verification
- `cargo test -p orbit-core`
- `cargo test --workspace`
- `cargo build -p orbit-cli --release`