# Fix: Use Pipeline Step Outputs Instead of Task Store for Automation Guards

**Goal:** Remove the `execution_summary` store dependency from `commit_task_changes` and `open_pr_from_task`. Both automations should read agent-produced content from their pipeline inputs.

**Scope:** `orbit-engine/src/executor/automation.rs`, the two activity YAML files (`commit_changes` and `open_pr`), and their canonical asset counterparts in `orbit-core/assets/activities/`. No changes to the `EngineHost` trait or any store.

**Assumptions:** `step_output_for_following_input` already merges each step's output into `current_input` for the following step. `implement_change`'s `result.summary` and `commit_task_changes`'s `commit_message`/`changed_files` are therefore available as flat input fields by the time the next step runs.

**Risks:** If `implement_change` returns an empty `summary`, the commit step will fail at the input guard — same user-visible behaviour, different error site.

## Task 1: Fix `commit_task_changes`

**File:**
- Modify: `orbit-engine/src/executor/automation.rs` — `commit_task_changes` function

**Steps:**
1. Read `summary` from the input: `let summary = input_string_field(input, "summary");`
2. Replace the `task.execution_summary.trim().is_empty()` guard with a check on `summary`:
   ```rust
   let summary = input_string_field(input, "summary").unwrap_or_default();
   if summary.trim().is_empty() {
       return Err(OrbitError::Execution(format!(
           "task '{}' commit_task_changes requires a non-empty summary from implement_change",
           task_id
       )));
   }
   ```
3. Include `summary` in the git commit message as the body (appended after a blank line below the title line). Update `task_commit_message` or inline to accept an optional body.
4. Remove any `host.get_task()` usage that was only present for the `execution_summary` guard; keep it if still needed for `task.title` or `task.task_type`.

**Done When:**
- `commit_task_changes` uses `input["summary"]` for the guard and appends it to the commit message body.
- `cargo test -p orbit-engine` passes.

## Task 2: Update `commit_changes` activity schema

**Files:**
- Modify: `.orbit/activities/active/commit_changes.yaml`
- Modify: `orbit-core/assets/activities/commit_changes.yaml`

**Steps:**
1. Add `summary` to `input_schema_json.properties` and `required`.
2. Update `commit_message` in `output_schema_json` description to note it now includes the implementation summary as body.

**Done When:**
- Both YAML files reflect the new input contract.

## Task 3: Fix `open_pr_from_task`

**File:**
- Modify: `orbit-engine/src/executor/automation.rs` — `open_pr_from_task` function

**Steps:**
1. Read `commit_message` and `changed_files` from input.
2. Build PR body:
   ```rust
   let body = format!(
       "## Changes\n{}\n\n## Files Changed\n{}",
       commit_message,
       changed_files.iter().map(|f| format!("- `{f}`")).collect::<Vec<_>>().join("\n")
   );
   ```
3. Replace `let body = task.execution_summary.clone();` with the above.
4. Remove the `execution_summary.trim().is_empty()` guard — PR body is now derived from commit output.
5. Keep `host.get_task()` for `task.title` and `task.branch` (still needed).

**Done When:**
- `open_pr_from_task` uses pipeline inputs for the PR body.
- `cargo test -p orbit-engine` passes.

## Task 4: Update `open_pr` activity schema

**Files:**
- Modify: `.orbit/activities/active/open_pr.yaml`
- Modify: `orbit-core/assets/activities/open_pr.yaml`

**Steps:**
1. Add `commit_message` (string) and `changed_files` (array of string) to `input_schema_json.properties` and `required`.
2. Update `body` in `output_schema_json` description to reflect the new format.

**Done When:**
- Both YAML files are consistent with the automation behaviour.

## Task 5: Add regression test

**File:**
- Modify: `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Add a test that runs the pipeline with a mock `implement_change` output that has `summary` populated but task store has no `execution_summary`.
2. Assert commit step succeeds.
3. Assert failure case: `summary` absent from input → commit step fails.

**Done When:**
- Test named `commit_task_changes_uses_summary_from_input` passes.

## Final Verification

```
cargo test -p orbit-engine
cargo test -p orbit-core --test job_runtime_behavior
cargo test --workspace
```