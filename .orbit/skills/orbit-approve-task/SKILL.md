---
name: orbit-approve-task
description: Use this skill when requested to review Orbit tasks for approval, record explicit human sign-off, or advance tasks through lifecycle gates, including explicit rejection.
---

# Orbit Approve Task

## Purpose

Use this skill to record explicit human approval or rejection after reviewing an Orbit task.

## Approval Gates

The decision commands auto-detect the current status:

- `proposed -> backlog`: sets `proposal_approved_by` and `proposal_decision_note`
- `proposed -> rejected`: sets `proposal_rejected_by` and `proposal_decision_note`
- `review -> done`: sets `review_approved_by` and `review_decision_note`
- `review -> rejected`: sets `review_rejected_by` and `review_decision_note`

Commands:

```bash
orbit task reject <id> --by "<identity_display_name>" --note "<note>" # reject
orbit task approve <id> --by "<identity_display_name>" --note "<note>" # approve
```

## Workflow

1. Find a single task with status `proposed` and another task with status `review`. If none exists, then your job is done.
2. Confirm the tasks are in `proposed` or `review`.
3. For `proposed` tasks, review the task carefully and determine whether the task is valid.
   - If the proposed task is valid, approve with explicit identity and a meaningful note - and ASSIGN the task to the person (from `orbit identity list`) you think is best suited to complete it. 
   - If not valid, reject with `orbit task reject <id> --by <identity_display_name> --note <reason>`.
4. For `review` tasks, confirm all requirements were fulfilled as outlined in the task.
   - If the task is completed successfully and you accept code changes, approve with `orbit task approve <id> --by <identity_display_name> --note <reason>` and include `result.commit` in the approval response so Orbit can create the commit.
   - A `review approved` result that accepts code changes must include `result.commit`; do not approve changed code without commit intent.
   - If the task is incomplete, reject with `orbit task reject <id> --by <identity_display_name> --note <reason>` and explain what still needs to be resolved.


## Verification Rules

- Before decision: identity, scope, and current status are correct.
- After proposal approval: status is `backlog`.
- After proposal rejection: status is `rejected`.
- After review approval: status is `done`.
- After review rejection: status is `rejected`.

## Output Requirements

Report:

- action taken (`proposal approved`, `proposal rejected`, `review approved`, or `review rejected`)
- task ID
- decision identity used
- decision note
- verification result

Keep output concise, operational, and auditable.

## Safety Rules

- Never approve the wrong task ID.
- Never reject the wrong task ID.
- Never infer approval from ambiguity.
- Prefer explicit `orbit task approve` and `orbit task reject` over implicit pathways.
- Record meaningful notes for auditability.
