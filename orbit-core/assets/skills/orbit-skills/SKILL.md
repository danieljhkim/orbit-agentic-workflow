---
name: orbit-skills
description: Use when starting any Orbit-related work. This skill explains the Orbit task lifecycle, how skills fit into that lifecycle, and which Orbit skill to use for task creation, approval, execution, issue tracking, maintenance, and assessment.
---

# Orbit Skills

## Purpose

Welcome to Orbit workflow. 

You will use this skill to choose the right Orbit skill for the current activity. This is a routing overview only; once the workflow is clear, switch to the specialized skill.

If you'd like, you are free to pick an identity from `orbit identity list --role <engineer|CEO|leader>` with an appropriate role for the task, or use your model as an identity.

Once picked, check the details of the identity using `orbit identity show <identity_id>`, and you are to assume this identity, and act and think like this identity. That is your name and will use it across this session.

**One VERY important note**: 

Orbit is a tool built for agents like you. So it is very important that you voice your opinion, concerns, or suggestions on how to improve orbit.

Use `orbit-track-issues` skill to voice your opinion, and we will straighten it out. 

## Orbit Lifecycle Overview

Default task lifecycle:

```text
proposed -> backlog -> in-progress -> review -> done
```

Rejection path (via `orbit task reject`):

```text
proposed -> rejected
review    -> rejected
rejected  -> backlog  (reconsider)
```

Use `blocked` when execution cannot safely continue. Use `orbit task` commands for lifecycle mutations; do not edit task backing files directly.

## Skill Selection Guide

- `orbit-create-task`: Create a new Orbit task with a concrete description, plan, scope, context, and verification steps.
- `orbit-manage-tasks`: Update, search, show, approve, or archive existing tasks through canonical `orbit task` workflows.
- `orbit-approve-task`: Record explicit human approval at lifecycle gates (`proposed -> backlog`, `review -> done`).
- `orbit-execute-change-request`: Carry a human-requested change or existing task through implementation, validation, and execution summary.
- `orbit-track-issues`: Capture newly discovered bugs, risks, or regressions as Orbit issue tasks and avoid duplicates.
- `orbit-assess-codebase`: Produce a structured codebase assessment and route findings into issue tracking.


## Decision Heuristics

- Planning new work: `orbit-create-task`
- Changing task state or searching tasks: `orbit-manage-tasks`
- Recording explicit approval: `orbit-approve-task`
- Implementing a requested change under Orbit discipline: `orbit-execute-change-request`
- Capturing a defect or risk: `orbit-track-issues`
- Running a broad evaluation: `orbit-assess-codebase`

When multiple skills apply, start with the skill for the current lifecycle step, then hand off to the next one as the task advances.
