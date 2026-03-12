# Job CLI Cleanup Plan

**Goal:** Remove the duplicate `orbit job archive` subcommand and change `orbit job delete` to move job files to a `disabled/` subdirectory.
**Scope:** CLI presentation layer (`orbit-cli`), file-based job store (`orbit-store`), and related tests. SQLite backend is not changed.
**Assumptions:** File-based backend is the path exercised by the changes; SQLite `mark_job_disabled` continues to work unchanged.
**Risks:** `list_jobs`, `get_job`, and `due_jobs` all call `read_all_activities()` — that method must scan the `disabled/` subdir when `include_disabled = true`, and skip it when false.

## Task 1: Remove `orbit job archive` subcommand

**Files:**
- Modify: `orbit-cli/src/command/job.rs`

**Steps:**
1. Add/confirm test that `orbit job archive` is not a valid subcommand (or confirm existing test coverage).
2. Remove `Archive(JobArchiveRunArgs)` variant from `JobSubcommand` enum.
3. Remove the `JobSubcommand::Archive` match arm.
4. Remove `JobArchiveRunArgs` struct and its `Execute` impl.
5. Run: `cargo test -p orbit-cli`
6. Run: `make ci`

**Done When:**
- `orbit job archive` is rejected as unknown subcommand.
- All existing CLI tests pass.

## Task 2: Move deleted job files to `disabled/` subdirectory

**Files:**
- Modify: `orbit-store/src/file/job_store.rs`

**Steps:**
1. Write a failing test: delete a job and assert the file no longer exists at `activities_dir/<id>.yaml` but does exist at `activities_dir/disabled/<id>.yaml`.
2. Add `fn disabled_jobs_dir(&self) -> PathBuf { self.activities_dir().join("disabled") }`.
3. Update `ensure_layout` to `fs::create_dir_all` the disabled dir.
4. Rewrite `mark_job_disabled`: read the job, move the file from `activities_dir/<id>.yaml` to `disabled_jobs_dir/<id>.yaml` (using `fs::rename`).
5. Update `get_job`: if not found in `activities_dir`, check `disabled_jobs_dir`.
6. Update `read_all_activities`: when `include_disabled = true`, also read YAMLs from `disabled_jobs_dir`.
7. Verify `due_jobs` is unaffected (it filters on `Enabled` state so disabled files won't appear anyway).
8. Run: `cargo test -p orbit-store`
9. Run: `make ci`

**Done When:**
- `orbit job delete <id>` results in the job YAML being present at `.orbit/jobs/jobs/disabled/<id>.yaml`.
- `orbit job list` does not show the deleted job.
- `orbit job list --all` (or equivalent) shows it.
- All store and CLI tests pass.

## Final Verification
```bash
make ci
cargo test -p orbit-store job_state
cargo test -p orbit-cli job
```