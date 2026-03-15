# Job Task Pipeline Workspace Propagation Plan

**Goal:** Ensure `job_task_pipeline` can run CLI steps like `create_branch`, `run_tests`, and `checkout_branch` with a valid workspace path.
**Scope:** Pipeline step input propagation, activity template context, and the affected built-in job/activity assets.
**Assumptions:** The correct workspace already exists on the selected task record; the bug is that the pipeline does not expose it where CLI activities need it.
**Risks:** A narrow fix for `create_branch` alone could leave later pipeline steps broken in the same way, so the solution should cover the whole pipeline path consistently.

## Task 1: Trace how workspace should flow through the pipeline

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Read/Modify: `orbit-core/src/template.rs`
- Read: `orbit-core/assets/jobs/job_task_pipeline.yaml`
- Read: `orbit-core/assets/activities/create_branch.yaml`
- Read: `orbit-core/assets/activities/run_tests.yaml`
- Read: `orbit-core/assets/activities/checkout_branch.yaml`

**Steps:**
1. Inspect how step outputs are merged into later step inputs today.
2. Determine where task workspace should enter the pipeline: dispatch output, job runtime augmentation, or activity-level lookup.
3. Confirm all pipeline steps that currently rely on `{{workspace_path}}`.

**Done When:**
- There is one clear propagation model for workspace path across the task pipeline.

## Task 2: Implement a consistent workspace-path fix

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Modify any directly affected activity/job assets in `orbit-core/assets/`
- Modify mirrored `.orbit` job/activity copies if the live workspace must keep working immediately

**Steps:**
1. Make the selected task workspace available to downstream pipeline steps.
2. Ensure `create_branch`, `run_tests`, and `checkout_branch` can render their working directory successfully.
3. Keep the fix consistent across bundled assets and the current live `.orbit` copies used by this repo.

**Done When:**
- `job_task_pipeline` no longer fails on missing `workspace_path` after dispatching a task.
- All affected CLI steps can resolve their working directory.

## Task 3: Add regression coverage for the pipeline path

**Files:**
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Modify any relevant asset/runtime tests if needed

**Steps:**
1. Add a test that reproduces the current failure after a dispatched task feeds into `create_branch`.
2. Assert the workspace path reaches later steps correctly.
3. Verify no later CLI pipeline step still depends on missing `workspace_path`.

**Done When:**
- Tests prove workspace propagation works across the pipeline path.

## Final Verification
- `cargo test -p orbit-core --test job_runtime_behavior`
- `cargo test --workspace`
- `orbit job run job_task_pipeline` only if a safe non-side-effect verification path exists