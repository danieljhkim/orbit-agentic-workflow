## Goal
Allow job definitions to declare default input values so autonomous scheduled runs do not require CLI `--input` flags for static configuration like the base branch.

## Scope
- Add `default_input: Option<serde_json::Value>` to `Job` struct and YAML schema
- Merge default_input as the initial `current_input` in the job runner (CLI `--input` takes precedence)
- Make `base` optional (not required) in `create_branch`'s `input_schema_json` with a note that it defaults to `"main"` — the runtime default is enforced by `job_task_pipeline`'s `default_input`, not by the activity itself
- Update `job_task_pipeline.yaml` (both built-in asset and `.orbit/` copy) to set `default_input: {base: main}`
- Update job store deserialization to handle missing `default_input` gracefully (serde default)

## Assumptions
- `default_input` is always a flat JSON object (type: object)
- CLI `--input` key=value pairs fully override (not merge with) default_input values for colliding keys
- The `Job` type is serialised to/from YAML; the new field must use `#[serde(default)]`

## Risks
- Existing job YAML files without `default_input` must still deserialise cleanly
- Changing `create_branch`'s required fields may affect direct activity invocations that previously relied on the schema for validation

## Task 1: Add `default_input` to Job struct and runner

**Files:**
- Modify: `orbit-types/src/job.rs` — add `#[serde(default)] pub default_input: Option<serde_json::Value>` to `Job`
- Modify: `orbit-core/src/command/job.rs` — in `execute_activity_with_retries`, initialise `current_input` by merging `job.default_input` first, then overlaying the caller-supplied `input`
- Modify: `orbit-store/src/file/job_store.rs` — verify deserialisation handles missing field (serde default should cover this)

**Steps:**
1. Write a failing test: create a job with `default_input: {base: "main"}` and a step that requires `base` — run without `--input` — confirm failure today.
2. Add field to `Job` struct with `#[serde(default)]`.
3. In the runner, seed `current_input` as `merge(job.default_input, caller_input)` (caller wins).
4. Re-run test — confirm pass.

## Task 2: Update YAML assets and make `base` optional in `create_branch`

**Files:**
- Modify: `orbit-core/assets/activities/create_branch.yaml` — remove `base` from `required` list in `input_schema_json`
- Modify: `orbit-core/assets/jobs/job_task_pipeline.yaml` — add `default_input: {base: main}`
- Modify: `.orbit/activities/active/create_branch.yaml` — same schema change
- Modify: `.orbit/jobs/jobs/job_task_pipeline.yaml` — same default_input addition

**Steps:**
1. Remove `base` from `required` in both `create_branch.yaml` files.
2. Add `default_input:\n  base: main` to both `job_task_pipeline.yaml` files.
3. Run: `orbit job run job_task_pipeline` (no `--input`) — confirm it reaches `create_branch` with `base=main`.

## Final Verification
```
cargo test --workspace
orbit job run job_task_pipeline
```