---
name: orbit
description: Entry point for Orbit workflow. Covers lifecycle, invocation patterns, identity, and skill routing. Load this for any Orbit-related work.
---

# Orbit

## Purpose

This skill orients agents working with Orbit. Orbit operations use two invocation patterns depending on whether a registered tool exists.

## Invocation Patterns

**Registered tools** — invoke via `orbit tool run`:

```bash
orbit tool run orbit.task.list --input '{"status": "backlog"}'
orbit tool run orbit.task.show --input '{"id": "T20260315-123456"}'
orbit tool run orbit.task.update --input '{"id": "T20260315-123456", "status": "review"}'
orbit tool run orbit.task.update --input '{"id": "T20260315-123456", "comment": "notes here"}'
```

**CLI-only operations** (no registered tool — run directly):

```bash
orbit task add --title "..." --description "..." --plan "..." --workspace "." --proposed-by "Name"
orbit task approve <id> --by "<identity_display_name>" --note "<note>"
orbit task reject <id> --by "<identity_display_name>" --note "<note>"
orbit identity list --role engineer|CEO|leader
orbit identity show <identity_id>
orbit job-run list --status failed
orbit job-run show <job_run_id>
orbit job-run archive <job_run_id>
```

Never edit task files directly.

## Lifecycle

```text
proposed → backlog → in-progress → review → done
```

Rejection path:

```text
proposed → rejected
review    → rejected
rejected  → backlog  (reconsider)
```

Use `blocked` when execution cannot safely continue.

## Identity

Run `orbit identity list --role engineer` (or `CEO|leader`), then `orbit identity show <id>`. Assume this identity for the session.

## Skill Selection

- `orbit-create-task`: Create a new task with plan, context, and verification steps.
- `orbit-approve-task`: Record human approval or rejection at lifecycle gates.
- `orbit-execute-change-request`: Carry a change through implementation, validation, and review.
- `orbit-maintain-system`: Routine maintenance and operational job-run audits.
- `orbit-track-issues`: Capture discovered bugs, risks, or friction as tracked tasks.

## Voice Your Opinion

Orbit is built for agents. If something is unclear, missing, or creates friction, track it with `orbit-track-issues`.
