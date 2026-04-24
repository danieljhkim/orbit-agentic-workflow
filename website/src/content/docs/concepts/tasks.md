---
title: Tasks
description: "How Orbit models durable work, state transitions, acceptance criteria, and review."
sidebar:
  order: 2
---

## Definition

A task is a durable unit of work stored in workspace-local Orbit state. It carries a title, description, acceptance criteria, lifecycle state, context, review notes, and audit history.

Use tasks for work an agent can execute and a human can review. Do not use a task as a scratch note when no observable outcome exists.

## Lifecycle

The common path is:

```text
proposed -> backlog -> in_progress -> review -> done
```

Human-created direct tasks may enter the backlog immediately. Proposed tasks must be approved before normal execution. Review tasks are approved or rejected after the agent produces work.

## Quality Bar

A good task states:

- what should change
- where the change should happen
- how to observe success
- which files or selectors matter when known

Acceptance criteria should be testable. Prefer "command X exits successfully" or "file Y contains Z" over "the behavior feels better."
