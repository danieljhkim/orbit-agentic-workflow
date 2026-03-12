## Status
success

## Orbit Task
Task ID: T20260310-065919-1773125959762575000

## 1. Summary of Changes
Fixed a duplicate-entry bug in the job file store introduced by the previous iteration:

**Root cause:** `set_job_state` called `write_activity` which always writes to the active `jobs/<id>.yaml` path. When a job is in `jobs/disabled/<id>.yaml` and the caller resumes it (setting state to `Enabled`), the new file was written to the active path but the disabled copy was never removed — producing two files for the same job ID.

**Fix in `orbit-store/src/file/job_store.rs` — `set_job_state`:**
- If the target state is `Disabled`, delegate entirely to `mark_job_disabled` (which already handles the move correctly)
- Otherwise, write to the active path as before, then remove `disabled_job_path` if it exists

**Test added** (`orbit-cli/tests/job_commands.rs`):
- `job_resume_after_delete_removes_disabled_copy`: deletes a job (moves to disabled/), resumes it, then asserts (a) active file exists, (b) disabled copy is gone, and (c) job appears exactly once in `job list --all` with state=enabled.

## 2. Strategic Decisions
- Delegate `set_job_state(Disabled)` to `mark_job_disabled` | Rationale: Single source of truth for disable logic; avoids future drift | Trade-offs: None
- Clean up disabled copy in `set_job_state` rather than `write_activity` | Rationale: Only `set_job_state` can know the job was previously disabled; `write_activity` is a low-level write primitive that should not carry location-awareness | Trade-offs: None

## 3. Assumptions Made
- Only the file backend is affected; SQLite backend does not use the `disabled/` filesystem path | Impact if incorrect: No impact — SQLite uses state column, no file moves

## 4. Design Weaknesses / Risks
- None introduced

## 5. Deviations from Original Plan
- Original task scope was to remove `orbit job archive` and move deleted jobs to `disabled/`; that was already done. This iteration fixes the reviewer-identified followup bug (duplicate on resume).

## 6. Technical Debt Introduced
- None

## 7. Recommended Follow-Ups
- None

## 8. Overall Assessment
Targeted one-method fix. Test confirms the exact failure scenario the reviewer described (delete + resume = duplicate) is no longer reproducible.