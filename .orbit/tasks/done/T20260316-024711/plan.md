# Job Engine Refactor Plan

**Goal:** Reduce `orbit-core/src/command/job.rs` to job command responsibilities and move execution logic into a dedicated internal engine/executor architecture inside `orbit-core`.
**Scope:** Job runtime orchestration, activity execution boundaries, legacy `created_file` behavior, module structure, and regression coverage.
**Assumptions:** We prefer clearer ownership boundaries over preserving the current file layout, and we are comfortable removing deprecated report auto-commit behavior outright. We will establish an internal `engine` module before considering a separate `orbit-engine` crate.
**Risks:** This touches core runtime flow, so weak boundaries or partial extraction could leave duplicate logic behind. Tests must prove behavior is preserved while the architecture improves.

## Task 1: Remove legacy created_file auto-commit behavior

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Modify: any agent/runtime types that still surface special `created_file` handling
- Modify: built-in activity/skill assets that advertise report auto-commit behavior
- Modify: related tests in `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Remove `execute_created_file_auto_commit()` and the runtime path that tracks `created_file` for post-run commits.
2. Remove agent-side special handling that treats `result.created_file` as a privileged runtime channel.
3. Update bundled/live activity and skill docs so Orbit no longer promises report auto-commit.
4. Replace or remove regression coverage that only exists for this deprecated behavior.

**Done When:**
- Orbit no longer auto-commits reports or any other agent-created file via `created_file`.
- No built-in asset or skill still documents that behavior.

## Task 2: Introduce an internal engine boundary for job runs and activity execution

**Files:**
- Add: `orbit-core/src/engine/mod.rs`
- Add: `orbit-core/src/engine/job_runner.rs`
- Add: `orbit-core/src/engine/activity_runner.rs`
- Add any focused helper modules needed for step-input merging or execution context
- Modify: `orbit-core/src/lib.rs`
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Define a dedicated internal engine layer for job-run lifecycle orchestration.
2. Move step iteration, run finalization, stale-run recovery, and shared attempt normalization into the engine layer.
3. Move activity-type dispatch into a dedicated activity runner rather than leaving it embedded in `command/job.rs`.
4. Keep `command/job.rs` as the command-facing surface that delegates to the engine.

**Done When:**
- Job runtime orchestration no longer lives primarily in `command/job.rs`.
- There is one clear internal engine entrypoint for executing job runs.

## Task 3: Tighten executor boundaries by activity type

**Files:**
- Add or modify: `orbit-core/src/executor/agent.rs`
- Modify: `orbit-core/src/executor/cli_command.rs`
- Modify: `orbit-core/src/executor/api.rs`
- Modify: `orbit-core/src/executor/automation.rs`
- Modify: `orbit-core/src/engine/activity_runner.rs`

**Steps:**
1. Give each activity type a focused executor module.
2. Keep executors responsible for executing one activity and returning a normalized outcome, not mutating job-run persistence directly.
3. Centralize `spec_type` dispatch in one activity-runner path.
4. Remove duplicated validation or dispatch logic left behind in command code.

**Done When:**
- Activity execution paths are modular and single-purpose.
- Executors do not own job-run lifecycle persistence.

## Task 4: Re-home step input propagation and validation logic

**Files:**
- Modify: `orbit-core/src/engine/activity_runner.rs`
- Add or modify helper module(s) for step-input merge behavior
- Modify: related tests in `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Move agent-vs-non-agent output merge rules into a dedicated helper or engine module.
2. Keep activity input/output schema validation at the engine/activity boundary.
3. Preserve current runtime behavior for step chaining while making the rules easier to inspect and test.

**Done When:**
- Step-input merge rules live in one place and are covered by explicit tests.
- Schema validation is not scattered across unrelated layers.

## Task 5: Add regression coverage for the new architecture

**Files:**
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Modify: `orbit-core/tests/asset_formatting.rs`
- Modify: any CLI/init tests affected by moved modules or updated assets

**Steps:**
1. Preserve runtime coverage for successful job runs, setup failures, automation activities, workspace propagation, and PR creation.
2. Add any focused unit coverage needed for new engine helpers.
3. Verify that removing `created_file` support does not leave stale tests or stale contracts.

**Done When:**
- The refactor is protected by targeted engine/runtime tests and full workspace validation.

## Future Follow-up (Not This Task)
- Reassess extracting the internal engine boundary into a separate `orbit-engine` crate after the module split stabilizes and cross-module dependencies are well understood.

## Final Verification
- `cargo test -p orbit-core --test job_runtime_behavior -- --nocapture`
- `cargo test -p orbit-core --test asset_formatting -- --nocapture`
- `cargo test --workspace`