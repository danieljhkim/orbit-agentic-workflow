---
name: orbit-approve-task
description: Use this skill when requested to review Orbit tasks for approval, record explicit human sign-off, or advance tasks through lifecycle gates.
---

# Orbit Approve Task

## Purpose

Use this skill to record explicit human approval or after you have completed the review of an orbit task. 

## Approval Gates

The `orbit task approve` command auto-detects the current status:

- `proposed -> backlog`: sets `proposal_approved_by` and `proposal_decision_note`
- `review -> done`: sets `review_approved_by` and `review_decision_note`

Use `orbit-manage-tasks` for general CLI workflows; this skill covers approval-specific checks.

## Workflow

1. Confirm you have the correct task ID and scope.
2. Confirm the task is in `proposed` or `review`.
3. Confirm approval is explicit, not inferred.
4. Approve with explicit approver identity and a meaningful note.
5. Verify the approval fields and resulting status.

## Verification Rules

- Before approval: identity, scope, and current status are correct.
- After proposal approval: status is `backlog`.
- After review approval: status is `done`.
- After approval: approver and note match the requested values.

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
