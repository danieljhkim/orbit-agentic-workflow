---
name: orbit-approve-task
description: Use this skill when requested to review Orbit tasks for approval, record explicit human sign-off, or advance tasks through lifecycle gates, including explicit rejection.
---

# Orbit Approve Task

## Purpose

Use this skill to record explicit human approval or rejection after reviewing an Orbit task.

## Approval Gates

The tool commands auto-detect the current status and record the decision in task history:

- `proposed -> backlog`: appends `proposal_approved`
- `proposed -> rejected`: appends `proposal_rejected`
- `review -> done`: appends `review_approved`
- `review -> rejected`: appends `review_rejected`

```bash
orbit tool run orbit.task.approve --input '{"id": "<id>", "note": "<note>", "comment": "<audit summary>"}'
orbit tool run orbit.task.reject --input '{"id": "<id>", "note": "<note>", "comment": "<audit summary>"}'
orbit tool run orbit.task.start --input '{"id": "<id>", "note": "<note>", "comment": "<audit summary>"}'
```

## Workflow

1. Run `orbit tool run orbit.task.list --input '{"status": "proposed"}'` and `'{"status": "review"}'`. If both are empty, your job is done.
2. For `proposed` tasks, review the task carefully.
   - If valid and execution should begin immediately: run `orbit tool run orbit.task.start --input '{"id": "<id>", "note": "<note>", "comment": "<audit summary>"}'`.
   - If valid but execution should remain queued: run `orbit tool run orbit.task.approve --input '{"id": "<id>", "note": "<note>", "comment": "<audit summary>"}'`.
   - If not valid: run `orbit tool run orbit.task.reject --input '{"id": "<id>", "note": "<reason>", "comment": "<audit summary>"}'`.
3. For `review` tasks, confirm all requirements were fulfilled as outlined in the task.
   - If complete and code changes are acceptable: run `orbit tool run orbit.task.approve --input '{"id": "<id>", "note": "<note>", "comment": "<audit summary>"}'`.
   - If incomplete: run `orbit tool run orbit.task.reject --input '{"id": "<id>", "note": "<what still needs resolving>", "comment": "<audit summary>"}'`.

## Verification

Prefer the returned payload from `approve`, `reject`, or `start` as your
default verification. If you need to confirm the canonical stored task record:

- Check status (all cases): `orbit tool run orbit.task.show --input '{"id": "<id>"}'`
- Check history (started case): `orbit tool run orbit.task.show --input '{"id": "<id>", "field": "history"}'`

Expected outcomes:

- After proposal approval: status is `backlog`
- After immediate proposal start: status is `in-progress`; history includes both `proposal_approved` and `started`
- After proposal rejection: status is `rejected`
- After review approval: status is `done`
- After review rejection: status is `rejected`

## Persisting Results

Include the brief audit summary in the same `approve`, `reject`, or `start` command via its `comment` field. That keeps the approval trail self-contained, even though `done` tasks can now accept follow-up comments later when you need to add cross-task notes.

```bash
orbit tool run orbit.task.approve --input '{
  "id": "<id>",
  "note": "<decision note>",
  "comment": "<action taken> — <decision note> — <verification result>"
}'
```

Include: action taken (`proposal approved`, `proposal rejected`, `review approved`, or `review rejected`), task ID, and decision rationale. Keep comments concise, operational, and auditable.

## Safety Rules

- Never approve the wrong task ID.
- Never reject the wrong task ID.
- Never infer approval from ambiguity.
- Record meaningful notes for auditability.
