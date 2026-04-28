---
title: Activity and Job YAML
description: "Reference shapes for schemaVersion 2 activity and job assets."
sidebar:
  order: 3
---

## Activity Envelope

```yaml
schemaVersion: 2
kind: Activity
metadata:
  name: example_activity
spec:
  type: shell
  description: Run an allowlisted shell command.
  input_schema_json:
    type: object
    properties: {}
  output_schema_json:
    type: object
    properties:
      status:
        type: string
```

## Activity Types

| Type | Required fields | v1 status |
|------|-----------------|-----------|
| `agent_loop` | `instruction`, optional `tools`, `provider`, `backend`, `model`, `max_iterations`, `wall_clock_timeout_seconds` | Supported. v1 only supports `backend: cli`; `backend: http` is preview-only. |
| `groundhog` | `instruction`, optional `tools`, `provider`, `model`, `max_iterations`, `attempt_budget_default` | Not in v1 release surface — depends on the HTTP transport. |
| `deterministic` | `action`, optional `config` | Supported. |
| `shell` | `program`, `allowed_programs`, optional `args`, `timeout_seconds`, `expected_exit_codes` | Supported. |

## Job Envelope

```yaml
schemaVersion: 2
kind: Job
metadata:
  name: example_job
spec:
  state: enabled
  max_active_runs: 1
  kind: workflow
  steps:
    - id: run_echo
      target: activity:shell_reference
```

## Step Bodies

Reference an activity:

```yaml
- id: review
  target: activity:agent_review_diff
```

Inline a full activity spec:

```yaml
- id: echo
  spec:
    type: shell
    program: echo
    args: [hello]
    allowed_programs: [echo]
```

Run branches in parallel:

```yaml
- id: parallel_review
  parallel:
    join: { mode: all }
    branches:
      - id: branch_a
        target: activity:review_a
      - id: branch_b
        target: activity:review_b
```

## Modifiers

Each step may include `when` and `retry`.

```yaml
retry:
  max_attempts: 3
  initial_backoff_ms: 500
  backoff_cap_ms: 5000
  backoff_strategy: exponential
```
