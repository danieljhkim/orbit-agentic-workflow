## Goal
Rename all task YAML fields from camelCase to snake_case with a hard cut-over, and manually rewrite the remaining live task bundles so the active `.orbit/tasks/` state stays compatible.

## Scope
- Rename task YAML field names in the Rust file-backed task schema to snake_case
- Update the task file writer to emit snake_case only
- Manually rewrite the remaining task bundles under `backlog`, `in_progress`, and `proposed` to snake_case
- No backward compatibility aliases for old camelCase task files
- No changes to job, activity, identity, or config YAML formats

## Assumptions
- Task bundles under `done`, `archived`, and `rejected` are gone and do not need migration support
- The only task bundles that must survive the cut-over are the ones currently under `backlog`, `in_progress`, and `proposed`
- The implementation can update code and rewrite live task files in the same change window

## Risks
- Manually missing even one live task bundle would leave Orbit unable to load that task after the cut-over
- Task update/write paths, tests, and ad hoc real data in `.orbit/tasks/` must all agree on the new field names immediately
- Any external tooling that still expects camelCase task YAML would need to be updated separately

## Fields to rename (camelCase -> snake_case)
- `contextFiles` -> `context_files`
- `workspacePath` -> `workspace_path`
- `acceptanceCriteria` -> `acceptance_criteria`
- `createdBy` -> `created_by`
- `assignedTo` -> `assigned_to`
- `proposedBy` -> `proposed_by`
- `proposalApprovedBy` -> `proposal_approved_by`
- `proposalRejectedBy` -> `proposal_rejected_by`
- `proposalDecisionNote` -> `proposal_decision_note`
- `reviewApprovedBy` -> `review_approved_by`
- `reviewRejectedBy` -> `review_rejected_by`
- `reviewDecisionNote` -> `review_decision_note`
- `activityId` -> `activity_id`
- `jobId` -> `job_id`
- `jobRunId` -> `job_run_id`
- `prNumber` -> `pr_number`
- `createdAt` -> `created_at`
- `updatedAt` -> `updated_at`

## Task 1: Update task schema and writer to snake_case only

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-store/src/file/task_store.rs`
- Modify: `orbit-store/src/backend/contracts.rs`

**Steps:**
1. Remove camelCase-oriented serde naming for task file persistence and switch task metadata fields to snake_case.
2. Update the task YAML writer to emit snake_case field names only.
3. Update tests so persisted task YAML assertions expect snake_case.

## Task 2: Manually rewrite surviving live task bundles

**Files:**
- Modify: `.orbit/tasks/backlog/*/task.yaml`
- Modify: `.orbit/tasks/in_progress/*/task.yaml`
- Modify: `.orbit/tasks/proposed/*/task.yaml`

**Steps:**
1. Enumerate every remaining live task bundle under `backlog`, `in_progress`, and `proposed`.
2. Manually rename every camelCase task metadata key in those files to snake_case.
3. Confirm no camelCase task metadata keys remain in the surviving live task bundles.

## Task 3: Validate the hard cut-over

**Steps:**
1. Run `cargo test --workspace`.
2. Run `orbit task list` against the real `.orbit/` directory.
3. Run `orbit task show <sample-id>` for representative tasks from active states.
4. Run one safe task mutation and confirm the rewritten file stays snake_case.

## Final Verification
```
cargo test --workspace
orbit task list
orbit task show <sample-id>
```