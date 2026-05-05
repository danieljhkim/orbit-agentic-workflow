---
title: Activities and Jobs
description: "How Orbit represents executable work units and workflow orchestration."
sidebar:
  order: 3
---

## Activity

An activity is a reusable execution unit. Schema v2 activities declare `schemaVersion: 2`, `kind: Activity`, metadata, and a typed `spec`.

Supported activity types:

| Type | Use |
|------|-----|
| `agent_loop` | Run an agent with an instruction, provider, backend, and tool allowlist. v1 only supports `backend: cli`, but the schema default for `backend:` is `http` — pin `cli` explicitly. |
| `groundhog` | Run checkpointed HTTP agent attempts with reset and retry behavior. *Not part of the v1 release surface — depends on the HTTP transport.* |
| `deterministic` | Run a registered deterministic action. |
| `shell` | Run an allowlisted shell program. |

## Job

A job is a workflow. It has schedule state, optional default input, concurrency limits, and ordered steps.

Step bodies can reference an activity, inline an activity spec, or compose control flow:

- `target: activity:<name>`
- `spec: ...`
- `parallel`
- `fan_out` and `fan_in`
- `loop`

## Why Both Exist

Activities make execution behavior reusable. Jobs make orchestration explicit. This keeps the dispatch surface inspectable and avoids hiding agent behavior inside code.
