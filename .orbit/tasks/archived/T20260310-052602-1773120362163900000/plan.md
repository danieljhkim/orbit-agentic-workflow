# Tiered Output Model Implementation Plan

**Goal:** Add `--ops` signal-tier output to task/job/activity list commands and externalize execution summaries to reduce agent token consumption.
**Scope:** CLI output layer and storage layer for execution summaries. No changes to core domain types unless necessary.
**Assumptions:** Existing `--json` flag continues to return full objects for backward compatibility. `--ops` is additive.
**Risks:** Changing execution_summary storage is a breaking change for any code that reads the field directly; must audit all callers.

## Task 1: Add `--ops` flag to `orbit task list`

**Files:**
- Modify: `orbit-cli/src/command/task.rs`
- Test: `orbit-cli/tests/task_commands.rs` (or equivalent integration test)

**Steps:**
1. Add failing test: `orbit task list --ops --json` returns only `id`, `title`, `type`, `status`, `priority` per task.
2. Run targeted test: `cargo test -p orbit-cli task_list_ops`
3. Add `ops: bool` field to `TaskListArgs`.
4. Add `task_to_signal_json` helper returning the five fields only.
5. Branch on `self.ops` in `TaskListArgs::execute`.
6. Re-run targeted test. Run `make ci`.

**Done When:**
- `orbit task list --ops --json` returns ≤6 fields per object
- `orbit task list --json` still returns full objects (no regression)

## Task 2: Add `--ops` flag to `orbit job list` and `orbit activity list`

**Files:**
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-cli/src/command/activity.rs`

**Steps:**
1. Add failing tests for each.
2. Apply the same `--ops` pattern from Task 1.
3. Signal fields for job: `job_id`, `target_id`, `state`, `next_run_at`
4. Signal fields for activity: `id`, `type`, `description`, `is_active`
5. Run `make ci`.

**Done When:**
- Both commands accept `--ops --json` and return minimal fields
- No regression on existing `--json` output

## Task 3: Externalize execution summaries

**Files:**
- Review: `orbit-core/src/command/task.rs`
- Review: `orbit-store/src/file/` (task persistence)
- Modify: task storage layer to write summary to file, store path reference
- Modify: `orbit-cli/src/command/task.rs` (show and update commands)
- Test: update relevant integration tests

**Steps:**
1. Audit all callers of `task.execution_summary` field.
2. Add failing test: after `orbit task update <id> --execution-summary <text>`, the content is stored at `.orbit/reports/<task-id>/execution-summary.md` and the task field contains the path.
3. Implement: write summary to file on update, store relative path in `execution_summary` field.
4. Update `task show` to read and print the file content when path is present.
5. Update `task_to_json` to return `execution_summary_path` instead of inline content.
6. Run `make ci`.

**Done When:**
- `orbit task show <id>` still displays execution summary content
- `orbit task list --json` no longer returns inline summary content (only path)
- All existing tests pass

## Final Verification
```bash
make ci
orbit task list --ops --json   # signal fields only
orbit task list --json         # full fields, no regression
orbit job list --ops --json
orbit activity list --ops --json
orbit task update <test-id> --execution-summary 'test summary'
ls .orbit/reports/<test-id>/execution-summary.md
orbit task show <test-id>      # displays summary content from file
```