# Signal-Tier Output Implementation Plan

**Goal:** Add `--ops` flag to task, job, and activity list commands returning minimal signal-only JSON.
**Scope:** CLI output layer only (`orbit-cli/src/command/`). Zero changes to orbit-core, orbit-store, or domain types.
**Assumptions:** `--ops` is additive and does not replace `--json`. Both flags may coexist; `--ops` takes precedence when set.
**Risks:** Low. Output-only change with no persistence or domain impact.

## Task 1: Add `--ops` to `orbit task list`

**Files:**
- Modify: `orbit-cli/src/command/task.rs`
- Test: `orbit-cli/tests/task_commands.rs` (add or extend)

**Steps:**
1. Add failing test: `orbit task list --ops` outputs JSON with exactly the fields `id`, `title`, `type`, `status`, `priority` and no others.
2. Run targeted test: `cargo test -p orbit-cli task_list_ops` — confirm red.
3. Add `ops: bool` to `TaskListArgs`.
4. Add `task_to_signal_json` helper in `task.rs` returning the five signal fields.
5. In `TaskListArgs::execute`, branch on `self.ops` before the existing `self.json` branch.
6. Re-run targeted test — confirm green. Run `make ci`.

**Done When:**
- `orbit task list --ops` emits minimal JSON
- `orbit task list --json` is unchanged

## Task 2: Add `--ops` to `orbit job list` and `orbit activity list`

**Files:**
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-cli/src/command/activity.rs`
- Test: extend existing integration tests for each

**Steps:**
1. Add failing tests for both commands verifying signal field sets.
2. Add `ops: bool` to `JobListArgs` and `ActivityListArgs`.
3. Add `job_to_signal_json` and `activity_to_signal_json` helpers.
4. Branch on `ops` before the `json` branch in each execute method.
5. Run `make ci`.

**Done When:**
- `orbit job list --ops` returns `job_id`, `target_id`, `state`, `next_run_at` only
- `orbit activity list --ops` returns `id`, `type`, `description`, `is_active` only
- No regression on existing `--json` output for either command

## Final Verification
```bash
make ci
orbit task list --ops
orbit task list --json         # full output unchanged
orbit job list --ops
orbit job list --json
orbit activity list --ops
orbit activity list --json
```