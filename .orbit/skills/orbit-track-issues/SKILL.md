---
name: orbit-track-issues
description: Use this skill when issues are identified by agents or humans. All issues must be tracked. Use this to track issues properly.
---

# Track Issues

## Purpose

Use this skill to maintain issue lifecycle discipline while synchronizing each issue with an Orbit task.

## Rules

- No pre-existing pending Orbit issue should already cover the same concern.
- The issue must be clearly defined.
- Impact, risks, and next actions must be explicit.
- Lifecycle state must reflect reality.
- This skill tracks issues; it does not implement product changes.

## Orbit Task Contract

Create and manage issue tasks with `orbit task` commands.

- Every identified issue must have one Orbit task with `--type issue`.
- The task description must state the problem and impact.
- The task plan or instructions must include concrete recommended next actions.
- Assign priority by risk: `low`, `medium`, or `high`.

Use `orbit-create-task` for canonical task creation details.

## Completion Standard

Tracking is complete when:

- No duplicate Orbit issue task exists for the same concern.
- The Orbit issue task matches the issue-tracking contract.
