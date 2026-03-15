---
name: orbit-track-issues
description: MUST use this skill when ANY issues, vagueness, confusion, or difficulties are encountered during Orbit-related work so the problem is captured as a task and improved for future agents.
---

# Orbit Skill: Track Issues

## Purpose

This skill ensures that **any friction, ambiguity, or failure encountered while using Orbit** is recorded as an Orbit task so it can be fixed later.

Orbit is designed to continuously improve. When agents encounter problems, they must **create tasks instead of silently working around them**.

Examples of issues worth tracking:

- unclear command behavior
- missing CLI functionality
- confusing schema or config
- documentation gaps
- repetitive manual steps
- fragile workflows
- unclear error messages
- unexpected runtime behavior

If something slows the agent down, it should be tracked.

---

# When To Use

Use this skill whenever you encounter:

- unclear Orbit command usage
- missing automation capability
- confusing workflow behavior
- undocumented behavior
- repetitive manual work
- system limitations
- unclear errors or logs

Do **not ignore friction**. Always create a task.

---

# Expected Outcome

A new Orbit task is created that clearly describes:

1. the problem
2. where it occurred
3. why it caused friction
4. a suggested improvement

The goal is to help future agents avoid the same issue.

---

# Task Creation Guidelines

Refer to `orbit-create-task` skill.

---

# Important Rules

- Do not silently ignore Orbit problems
- Do not implement large design changes without a task
- Always document the root issue clearly

Orbit improves through tracked issues.

Future agents depend on these tasks.

