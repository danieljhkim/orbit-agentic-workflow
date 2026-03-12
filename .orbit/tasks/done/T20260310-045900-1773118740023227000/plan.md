# resolve-backlogged-task task_id Input Plan

**Goal:** Let operators manually run resolve-backlogged-task for a specific Orbit task using a simple task_id argument.
**Scope:** Add an ergonomic single-task identifier input path for resolve-backlogged-task job execution; do not redesign jobs to target tasks directly.
**Assumptions:** The existing resolve-backlogged-task activity remains the canonical execution target, and manual job runs should continue to flow through the current job-run pipeline.
**Risks:** A task-specific CLI shortcut can become an inconsistent one-off if the activity input contract and job-run contract are not defined clearly; validation must stay explicit so bad input does not silently fall back to backlog scanning behavior.

## Task 1: Define the input contract for resolve-backlogged-task

**Files:**
- Modify: orbit-core/assets/activities/resolve-backlogged-task.yaml
- Modify: orbit-core/src/command/activity.rs if seeded activity loading or validation needs adjustment
- Test: activity or runtime coverage for the updated input contract

**Steps:**
1. Update the resolve-backlogged-task activity input schema so task_id is accepted as the intended focused-run input.
2. Decide whether task_id is required for the focused-run path or optional with documented fallback behavior.
3. Ensure the contract stays explicit and agent-facing instructions reflect the new task-specific execution mode.
4. Add regression coverage for the updated activity input expectations.

**Done When:**
- resolve-backlogged-task has a clear, documented input contract for a direct task_id-driven run.

## Task 2: Add ergonomic CLI support for manual runs

**Files:**
- Modify: orbit-cli/src/command/job.rs
- Modify: orbit-core/src/command/job.rs
- Test: orbit-cli/tests/job_commands.rs
- Test: orbit-core/tests/job_runtime_behavior.rs

**Steps:**
1. Add a simple CLI argument for manual job runs that supplies a task_id for resolve-backlogged-task without requiring a raw JSON object.
2. Thread that input through the existing job-run execution envelope so the activity receives the expected task_id input.
3. Validate the input and produce actionable errors when the argument is missing, malformed, or incompatible with the target activity.
4. Add end-to-end coverage for successful targeted runs and invalid-input failure cases.

**Done When:**
- Operators can run an existing resolve-backlogged-task job with a simple task_id argument from the CLI.

## Task 3: Update docs and examples

**Files:**
- Modify: README.md
- Modify: examples.md
- Modify: CLI contract docs if needed

**Steps:**
1. Document the new manual-run syntax for resolve-backlogged-task.
2. Update examples so focused task execution is easy to discover.
3. Verify documentation matches the implemented command behavior.

**Done When:**
- The new task_id-based manual-run workflow is documented and discoverable.

## Final Verification
- cargo test -p orbit-core job_runtime_behavior
- cargo test -p orbit-cli job_commands
- cargo test -p orbit-core
- cargo test -p orbit-cli