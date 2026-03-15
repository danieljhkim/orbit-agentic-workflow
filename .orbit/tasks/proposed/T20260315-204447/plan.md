# Task History Audit Trail Plan

**Goal:** Make task history entries in `task.yaml` useful to humans by replacing vague lifecycle events like `moved` with status-aware audit information.
**Scope:** Task history persistence, task lifecycle mutation paths, and any CLI/runtime tests that assert task history behavior.
**Assumptions:** Orbit should preserve compact task history, but each entry should be descriptive enough to explain what changed without cross-referencing the current task status.
**Risks:** Changing the history entry shape could affect existing task loading or tests, so the implementation should keep the persisted format coherent and validate representative lifecycle flows.

## Task 1: Define a clearer history event contract

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-store/src/file/task_store.rs`
- Modify: `orbit-core/src/command/task.rs`

**Steps:**
1. Inspect how task history entries are modeled and written today.
2. Choose a more informative contract for lifecycle history entries.
3. Prefer a design that captures status transitions explicitly, such as from/to status fields or a clearer event name/payload.

**Done When:**
- The history model can express what changed during a task lifecycle move without relying on guesswork.

## Task 2: Update lifecycle writes and compatibility handling

**Files:**
- Modify: `orbit-store/src/file/task_store.rs`
- Modify: `orbit-core/src/command/task.rs`
- Modify any directly affected serialization/tests discovered during implementation

**Steps:**
1. Update task create/update/approve/reject/archive flows to write the richer history entries.
2. Ensure existing task bundles still load safely, or migrate them deliberately if the final design requires it.
3. Keep the resulting YAML readable in raw `task.yaml` form.

**Done When:**
- New task history entries clearly describe status changes and other important lifecycle actions.
- Orbit can still read persisted task bundles without ambiguity.

## Task 3: Add regression coverage for representative transitions

**Files:**
- Modify: `orbit-store/src/file/task_store.rs` tests
- Modify: `orbit-core` or `orbit-cli` task lifecycle tests as needed

**Steps:**
1. Add coverage for common transitions like `proposed -> backlog`, `in-progress -> review`, and `review -> done`.
2. Assert that the persisted history shows the actual transition detail rather than a generic `moved` event.
3. Run targeted and broader task-related validation.

**Done When:**
- Tests prove the history entries are more informative across normal lifecycle flows.

## Final Verification
- `cargo test -p orbit-store`
- `cargo test -p orbit-core`
- `cargo test -p orbit-cli --test task_commands`