---
name: orbit-execute-change-request
description: Use this when executing human-initiated code change or existing orbit task in order to manage the full lifecycle in Orbit tasks (create, update, archive). Use this when the user specifically instructs you to use "orbit skill".
---

# Orbit Execute Change Request

## Purpose

Handle a human-requested engineering change or existing Orbit task from intent to verified implementation, with explicit task lifecycle tracking.

## Responsibilities

1. Clarify intent and success criteria.
2. Create or link the tracking task in Orbit. If creating, use `orbit-create-task`.
3. Ensure `proposed` work is explicitly approved before execution.
4. Mark in-progress before making changes:
   ```bash
   orbit tool run orbit.task.update --input '{"id": "<task-id>", "status": "in-progress"}'
   ```
5. Implement and validate the change according to the task plan.
6. Move to review after validation:
   ```bash
   orbit tool run orbit.task.update --input '{"id": "<task-id>", "status": "review"}'
   ```
7. Persist the execution summary (see below).

See the `orbit` skill for full invocation patterns and lifecycle facts.

## Lifecycle Rules

- Manage a single Orbit task per change request.
- If material ambiguity remains, ask clarifying questions before implementation.
- If approval cannot be obtained for `proposed` work, stop after recording that state.
- Do not skip lifecycle updates.

## Output

Persist the execution summary via the CLI — do NOT write to a file path directly:

```bash
orbit task update <task-id> --execution-summary "$(cat <<'EOF'
<summary content>
EOF
)"
```

The CLI resolves the correct bundle path automatically. Never hardcode or guess the file location.

Use this structure for the execution summary:

```markdown
# Execution Summary - <Change Request Title>
Agent Name: <identity_display_name>
Agent Model: <model name>

## Status
success | failed

## Orbit Task
Task ID: <orbit-task-id>

## 1. Summary of Changes
<what changed>
## 2. Strategic Decisions
- <decision> | Rationale: <why> | Trade-offs: <trade-offs>
## 3. Assumptions Made
- <assumption> | Impact if incorrect: <impact>
## 4. Design Weaknesses / Risks
- <risk> | Severity: Low / Medium / High | Mitigation: <mitigation>
## 5. Deviations from Original Plan
- <deviation> | Justification: <why>
## 6. Technical Debt Introduced
- <item> | Recommended resolution: <next step>
## 7. Recommended Follow-Ups
- <next step>
## 8. Overall Assessment
<short quality assessment>
```

## Exit Criteria

- Requested change implemented
- Validation completed
- Task approved before execution, if required
- Task advanced through `review`
- Execution summary persisted via `orbit task update --execution-summary`
