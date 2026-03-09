---
name: orbit-approve-task
description: Use this skill when requested to review Orbit tasks for approval, record explicit human sign-off, or advance tasks through lifecycle gates, or `archive` the task if rejected.
---

# Orbit Approve Task

## Purpose

Use this skill to record explicit human approval OR after you have completed the review of an orbit task. 

## Approval Gates

The `orbit task approve` command auto-detects the current status:

- `proposed -> backlog`: sets `proposal_approved_by` and `proposal_decision_note`
- `proposed -> archived`: sets `proposal_approved_by` and `proposal_decision_note`
- `review -> done`: sets `review_approved_by` and `review_decision_note`
- `review -> backlog`: sets `review_decision_note` explaining why

Commands:

```bash
orbit task archive <id> # reject
orbit task approve <id> --by "<identity_display_name_or_model_name>" --note "<note>" # approve
```

## Workflow

1. Find all tasks with status `proposed` or `review`.
2. Confirm all the tasks are in `proposed` or `review`.
3. For `proposed` tasks, review the task carefully, and determine whether the task is valid.
    - If the proposed task is valid, approve with explicit approver identity and a meaningful note
    - If not valid, archive the task using `orbit task archive <id>`
4. For `review` tasks, review each task carefully and confrim that all the requirements were fulfilled as outlined in the task. 
    - If the task is completed successfully, update the task status to `done` - `orbit task close <id>`
    - If the task is incompleted,  update the task status to `backlog` and leave a note on why the task is incomplete. 


## Verification Rules

- Before approval: identity, scope, and current status are correct.
- After proposal approval: status is `backlog`.
- After proposal rejection: status is `archived`.
- After review approval: status is `done`.
- After review rejection: status is `backlog`.

## Output Requirements

Report:

- action taken (`proposal approved` or `review approved`)
- task ID
- approver identity used
- approval note, if any
- verification result

Keep output concise, operational, and auditable.

## Safety Rules

- Never approve the wrong task ID.
- Never infer approval from ambiguity.
- Prefer explicit `orbit task approve` over implicit pathways.
- Record meaningful notes for auditability.
