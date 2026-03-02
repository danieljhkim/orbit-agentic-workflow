---
name: orbit-execute-change-request
description: Use this when executing human-initiated code change in order to manage the full lifecycle in Orbit tasks (create, update, close). Use this before and after making any code changes.
---

# Orbit Execute Change Request

## Purpose

Handle human-initiated engineering changes (feature, refactor, improvement, issue) from intent to verified implementation, with explicit task lifecycle tracking in `orbit task`.

---

## Required Task Lifecycle

Manage a SINGLE Orbit task per change request:

1. Create task at start, if task is not already created.
2. If any doubt remains, ask clarifying questions and record them in the task.
3. Ensure task is approved before implementation or agent execution.
4. Update task during/after execution with implementation comments.
5. Close task when job is complete and validated.

Do not skip lifecycle updates.

---

## Inputs

- Natural-language change request
- Constraints (scope, files, deadlines), if any
- Repository workspace
- Priority/type hints, if any
- Actor identity metadata (identity id + display name), if available

---

## Responsibilities

1. Clarify intent and success criteria.
2. Create or link the tracking task in Orbit.
3. Obtain/record approval before execution.
4. Implement a minimal, safe diff.
5. Run validation (build/tests/lint).
6. Update and close the task with an auditable summary.
7. Persist an execution summary markdown file.

---

## Task Management Contract (`orbit task`)

### 1) Create Task (required before edits)

Use `orbit task add` and include at least:

- `--title` from the change request
- `--description` summarizing the requested outcome
- `--instructions` capturing initial plan/constraints
- `--workspace` current repository path
- `--type` appropriate to request (`task|feature|issue|other`)
- Attribution: `--assigned-to`, `--created-by`, and `--identity` when an identity id is available

Example:

```bash
orbit task add \
  --title "Refactor task lifecycle handling" \
  --description "Apply requested refactor and preserve behavior" \
  --instructions "Implement minimal diff; run tests" \
  --workspace "/abs/path/to/repo" \
  --identity "linus" \
  --assigned-to "Linus Torvalds (Maintainer)" \
  --created-by "Linus Torvalds (Maintainer)" \
  --type feature \
  --priority medium
```

Capture and retain the created task ID.

Task attribution rule (required):
- If identity is available, use identity id + display name for attribution.
- If identity is not available, use model name fallback for `--assigned-to` and `--created-by`.
- Set `--identity` only when an identity id exists (identity id or model-alias identity).
- Never leave `assigned_to` or `created_by` null on newly created tasks.

### 2) Approve Task (required before implementation)

Preferred explicit approval:

```bash
orbit task approve <TASK_ID> --by "human" --note "approved verbally by user"
```

Flexible path when user gives explicit verbal approval in-session:

```bash
orbit agent run --task <TASK_ID> --approve-on-verbal --approved-by "agent" --approval-note "approved based on explicit user verbal confirmation"
```

If task approval is required by config (`[task.approval] required_for_agent = true`), execution must not proceed without one of the above.

### 3) Update Task With Execution Comments (required)

After implementation/validation, update the task with execution comments.

Because `orbit task` has no dedicated comment command, write execution notes into `--instructions` and/or `--description`.

Include:

- What changed
- Files touched
- Validation results
- Risks/follow-ups

Example:

```bash
orbit task update <TASK_ID> \
  --instructions "Execution comments: updated parser, added tests, build/tests pass." \
  --status in-progress
```

Then apply final update reflecting completion summary.

If `identity_id`, `assigned_to`, or `created_by` are missing on an existing task, backfill them:

```bash
orbit task update <TASK_ID> \
  --assigned-to "<identity_display_name_or_model_name>" \
  --created-by "<identity_display_name_or_model_name>"
```

If an identity id is available, include it in the same update:

```bash
orbit task update <TASK_ID> --identity "<identity_id_or_model_identity>"
```

### 4) Close Task (required when done)

When acceptance criteria are met and validation is complete:

```bash
orbit task close <TASK_ID>
```

Verify final state:

```bash
orbit task show <TASK_ID>
```

---

## Execution Contract

- Operate only within the specified workspace.
- Prefer incremental, reviewable diffs.
- Preserve existing behavior unless explicitly changed.
- Fail fast on validation errors.
- Keep task state synchronized with actual execution state.


---

## Output

Persist an execution summary markdown file at:

```
{{ORBIT_ROOT}}/agents/executions/YYYY-MM-DD-<title>.md
```

The execution summary file must include:

- Change request title
- Linked Orbit task ID
- Summary of implementation
- Files modified
- Validation results (build/tests/lint)
- Risks, follow-ups, and notes

Return output as markdown (not JSON), using this structure:

```markdown
# Execution Summary - <Change Request Title>

## Status
success | failed

## Orbit Task
Task ID: <orbit-task-id>

## Summary
<what changed>

## Files Modified
- <file path>

## Validation
- Build: pass | fail | skipped
- Tests: pass | fail | skipped
- Lint: pass | fail | skipped

## Notes
<execution comments, follow-ups, risks>
```

---

## Exit Criteria

- Requested change implemented
- Validation completed
- Task approved before execution
- Task updated with execution comments
- Task closed (`done`) when successful
- Markdown summary written to `{{ORBIT_ROOT}}/agents/executions/YYYY-MM-DD-<title>.md`
