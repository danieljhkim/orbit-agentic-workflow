# Duration Parsing Deduplication Plan

**Goal:** Remove duplicate `parse_duration_seconds` implementations; replace with a single canonical version.
**Scope:** `orbit-cli` crate only unless callers exist in other crates. No behavior change.
**Assumptions:** The two implementations are identical in behavior. Verify before merging.
**Risks:** If a third caller exists outside `orbit-cli`, the canonical location must move to a shared crate (e.g., `orbit-exec` or `orbit-types`). Check first.

## Task 1: Audit all callers

**Files:**
- Search: `orbit-cli/src/` for `parse_duration_seconds`
- Search: workspace-wide for `parse_duration_seconds`

**Steps:**
1. Run `grep -r parse_duration_seconds .` across the workspace.
2. Confirm whether callers exist outside `orbit-cli`.
3. If callers are only in `orbit-cli`: canonical location is `orbit-cli/src/parse.rs`.
4. If callers exist in other crates: canonical location is `orbit-exec` or `orbit-types`; note this as a deviation.

**Done When:**
- All call sites are identified and their crate locations recorded.

## Task 2: Extract to canonical location

**Files:**
- Read first: `orbit-cli/src/parse.rs` (check existing parse utilities)
- Modify: `orbit-cli/src/parse.rs` (add `pub fn parse_duration_seconds`)
- Modify: `orbit-cli/src/command/activity.rs` (remove local function, use `crate::parse::parse_duration_seconds`)
- Modify: `orbit-cli/src/command/job.rs` (same)
- Modify: `orbit-cli/src/command/job_run.rs` if it also has a copy

**Steps:**
1. Add a test for `parse_duration_seconds` in `orbit-cli/src/parse.rs` covering: `30s`, `5m`, `2h`, `1d`, `1w`, empty string, invalid unit, non-numeric prefix.
2. Run targeted test: `cargo test -p orbit-cli parse_duration`
3. Move the function body to `orbit-cli/src/parse.rs`, make it `pub`.
4. Remove both local definitions.
5. Update both call sites to `crate::parse::parse_duration_seconds`.
6. Run `make ci`.

**Done When:**
- Exactly one definition of `parse_duration_seconds` exists in the workspace
- All existing tests pass
- `make ci` is green

## Final Verification
```bash
grep -r 'fn parse_duration_seconds' .   # should return exactly 1 result
make ci
```