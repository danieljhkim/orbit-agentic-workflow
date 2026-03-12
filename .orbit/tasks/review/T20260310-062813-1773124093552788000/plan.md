## Fix Plan

**Goal:** Correct the wrong assertion in the test so it matches the documented default.
**Scope:** One line change in `orbit-cli/tests/job_commands.rs`. No production code changes.
**Assumptions:** `15m` → 900 seconds is the intended default and is correct.
**Risks:** None — test-only change.

## Steps

1. Open `orbit-cli/tests/job_commands.rs` line 179.
2. Change `assert_eq!(show["timeout_seconds"], 7000);` to `assert_eq!(show["timeout_seconds"], 900);`.
3. Run `cargo test -p orbit job_add_defaults_timeout_to_fifteen_minutes` → must pass.
4. Run `make ci` → all tests pass.

## Done When
- `job_add_defaults_timeout_to_fifteen_minutes` passes
- `make ci` passes (no regressions)