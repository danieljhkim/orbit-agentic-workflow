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

# How to Create the Task

```bash
orbit tool run orbit.task.add --input '{
  "title": "<short, specific problem statement>",
  "description": "<what happened, where, and why it caused friction>",
  "plan": "<suggested fix or investigation steps>",
  "workspace": ".",
  "type": "issue",
  "priority": "<low|medium|high|critical>"
}'
```

Required fields: `title`, `description`, `plan`, `workspace`. Keep the description concrete — name the command, file, or workflow that broke.

---

# Important Rules

- Do not silently ignore Orbit problems — always create a task.
- Do not implement large design changes inline — track them first.
- Document the root issue clearly so the next agent can act on it.

