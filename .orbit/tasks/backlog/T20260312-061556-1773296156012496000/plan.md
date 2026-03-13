# Multi-Activity Job Chaining Plan

**Goal:** Refactor Orbit jobs so one job can run multiple activities in a fixed order, with each step feeding the next.
**Scope:** Job model, persistence, runtime execution, CLI/user-facing definition, seeded/default jobs, and targeted regression coverage.
**Assumptions:** The initial chaining use case is linear orchestration such as `resolve-backlogged-task -> approve-task-leader`, and v1 can stop on the first non-success outcome.
**Risks:** If backward compatibility is weak, existing persisted jobs may break. If step input/output rules are ambiguous, chained activities will be hard to reason about. If job-run artifacts do not capture per-step outcomes, debugging chained jobs will be painful.

## Task 1: Redesign the job schema around ordered activities

**Files:**
- Modify: `orbit-types/src/job.rs`
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/file/job_store.rs`
- Modify: `orbit-store/src/sqlite/job_store.rs`
- Modify: `orbit-store/src/sqlite/migration.rs`
- Test: `orbit-store/tests/job_store_integration.rs`

**Steps:**
1. Replace or extend the single `target_type` / `target_id` job shape with an ordered `activities` collection.
2. Define the minimal per-step structure for v1, such as `activity_id` and any metadata required for persistence.
3. Decide and implement the backward-compatibility path for existing single-activity jobs stored in file and sqlite backends.
4. Update serialization, deserialization, and migrations so persisted jobs can round-trip cleanly.
5. Add store-level tests for reading and writing both chained jobs and legacy single-step jobs.

**Done When:**
- Job definitions can persist ordered activity chains.
- Existing stored jobs still load correctly or migrate deterministically.

## Task 2: Redesign job-run artifacts to capture per-step execution

**Files:**
- Modify: `orbit-types/src/job.rs`
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/file/job_store.rs`
- Modify: `orbit-store/src/sqlite/job_store.rs`
- Test: `orbit-store/tests/job_store_integration.rs`

**Steps:**
1. Add a typed per-activity execution record to job-run persistence, including activity identity, order, state, timing, and response/error details.
2. Decide what remains as the top-level job-run summary versus what lives per step.
3. Ensure persisted artifacts clearly show partial progress when a later step fails.
4. Add coverage proving chained run artifacts serialize and deserialize correctly.

**Done When:**
- Job-run artifacts tell the full step-by-step story for chained execution.
- Operators can inspect which activity failed without guesswork.

## Task 3: Execute activity chains sequentially in runtime

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Modify: `orbit-core/src/command/activity.rs` if shared helpers need to move
- Test: `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Refactor job execution to iterate through ordered activities instead of resolving exactly one activity target.
2. Define the v1 input propagation rule clearly:
   - first step input = job run input
   - later step input = previous step's successful `result`
3. Stop execution immediately on the first failed, timeout, or protocol-violation step.
4. Persist each step's outcome into the job-run artifact as execution progresses.
5. Add runtime tests for success chains, fail-fast behavior, and step-to-step result propagation.

**Done When:**
- Chained jobs run activities in order only.
- The resolve-to-approve case can be represented without a special-case runtime hook.

## Task 4: Expose chained jobs through CLI and seeded defaults

**Files:**
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-cli/tests/job_commands.rs`
- Modify: `orbit-core/src/command/job.rs`
- Modify: `.orbit/jobs/jobs/job-resolve-backlogged-task.yaml`
- Modify: `.orbit/jobs/jobs/job-approve-task-leader.yaml`
- Modify: any new/updated seeded chained job definitions under `.orbit/jobs/jobs/`
- Modify: `orbit-cli/tests/init_commands.rs`

**Steps:**
1. Add a user-facing way to create or inspect multi-activity jobs, such as repeated `--activity <id>` flags or equivalent JSON/YAML support.
2. Keep single-activity job creation ergonomic, either by treating it as a one-step chain or preserving compatible CLI aliases.
3. Seed or document the resolve-to-approve workflow as a chained job instead of two manually coordinated jobs.
4. Update init/default coverage so repo-local seeded jobs and assets remain consistent.

**Done When:**
- Users can define chained jobs intentionally.
- The example resolve-to-approve workflow is data-driven through the new model.

## Task 5: Verify operator-facing behavior and documentation assumptions

**Files:**
- Test: `orbit-core/tests/job_runtime_behavior.rs`
- Test: `orbit-cli/tests/job_commands.rs`
- Test: `orbit-cli/tests/init_commands.rs`
- Modify: minimal docs/examples if needed to explain ordered chaining

**Steps:**
1. Add end-to-end style regression coverage for a two-step chained job.
2. Add negative-path coverage for invalid activity IDs, incompatible propagated input, and partial-step failures.
3. Verify job inspection output remains understandable when runs contain multiple activity results.
4. Run targeted verification commands.

**Done When:**
- Chained jobs are covered by focused happy-path and failure-path tests.
- Operator output stays auditable and understandable.

## Final Verification
- `cargo test -p orbit-store -- --nocapture`
- `cargo test -p orbit-core --test job_runtime_behavior -- --nocapture`
- `cargo test -p orbit --test job_commands -- --nocapture`
- `cargo test -p orbit --test init_commands -- --nocapture`
- Manual spot check if practical: create a two-step job for `resolve-backlogged-task` then `approve-task-leader`, run it with a task id, and confirm the job-run artifact records both steps in order.