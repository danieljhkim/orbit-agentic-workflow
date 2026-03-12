# Activity Update Command Implementation Plan

**Goal:** Add `orbit activity update <id> [flags]` with patch semantics for all mutable activity fields.
**Scope:** CLI arg definition, core domain command, store persistence. No changes to the Activity struct's identity fields (id, created_at).
**Assumptions:** Patch semantics — only supplied flags change the record. Consistent with `TaskUpdateParams` pattern already in the codebase.
**Risks:** Store layer may not have an update path for activities; may need to add it.

## Task 1: Add ActivityUpdateParams to orbit-core

**Files:**
- Read first: `orbit-core/src/command/activity.rs`
- Read first: `orbit-core/src/command/task.rs` (reference pattern for UpdateParams)
- Modify: `orbit-core/src/command/activity.rs`
- Modify: `orbit-core/src/lib.rs` (expose `update_activity` on OrbitRuntime if not present)
- Modify: `orbit-store/src/file/` (add update path for activity store if missing)
- Test: `orbit-core/tests/` (add activity update test)

**Steps:**
1. Read `orbit-core/src/command/task.rs` to understand the TaskUpdateParams pattern.
2. Read `orbit-core/src/command/activity.rs` to see current structure.
3. Add failing test: call `runtime.update_activity(id, params)` and assert field changes.
4. Define `ActivityUpdateParams` struct with `Option` fields for each mutable attribute.
5. Implement `update_activity` on OrbitRuntime applying patch semantics.
6. Add store-level update if the activity store only supports write-on-create.
7. Run targeted test: `cargo test -p orbit-core activity_update`.

**Done When:**
- `runtime.update_activity()` exists and passes tests
- Unmutated fields retain their original values

## Task 2: Add ActivityUpdateArgs to orbit-cli

**Files:**
- Read first: `orbit-cli/src/command/activity.rs`
- Modify: `orbit-cli/src/command/activity.rs`
- Test: `orbit-cli/tests/activity_commands.rs` (or equivalent)

**Steps:**
1. Add failing integration test: `orbit activity update <id> --description 'new desc'` exits 0 and `orbit activity show <id>` reflects the change.
2. Add `Update(ActivityUpdateArgs)` variant to `ActivitySubcommand`.
3. Define `ActivityUpdateArgs` with `pub id: String` and optional flag fields mirroring `ActivityUpdateParams`.
4. Implement `Execute for ActivityUpdateArgs` calling `runtime.update_activity()`.
5. Handle empty-string-clears semantics for nullable fields (same pattern as TaskUpdateArgs).
6. Print `Updated activity '<id>'` on success.
7. Run `make ci`.

**Done When:**
- `orbit activity update <id> --description 'x'` succeeds
- `orbit activity show <id>` reflects the new description
- Other fields are unchanged
- `orbit activity update <id> --identity ''` clears the identity field

## Final Verification
```bash
make ci
orbit activity add --id test-update-001 --description 'original' --type general
orbit activity update test-update-001 --description 'updated'
orbit activity show test-update-001   # description == 'updated', created_at unchanged
orbit activity update test-update-001 --assigned-to 'Grace'
orbit activity show test-update-001   # assigned_to set
orbit activity update test-update-001 --assigned-to ''
orbit activity show test-update-001   # assigned_to cleared
```