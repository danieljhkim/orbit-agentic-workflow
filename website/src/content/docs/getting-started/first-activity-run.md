---
title: First Activity Run
description: "Run a schemaVersion 2 activity YAML file directly from the Orbit CLI."
sidebar:
  order: 4
---

## List Activities

Orbit ships default activities. List them before running one directly.

```bash
orbit activity list
orbit activity list --json
```

Use `--ops` for the compact signal shape.

```bash
orbit activity list --ops
```

## Run an Activity YAML

Run a checked-in activity file:

```bash
orbit activity run crates/orbit-core/assets/activities/worktree_setup.yaml
```

Pass JSON input when the activity expects it:

```bash
orbit activity run path/to/activity.yaml --input '{"task_id":"T123"}'
```

## Choose a Backend

For `agent_loop` activities, backend resolution follows this order:

1. `--backend`
2. `ORBIT_BACKEND`
3. `[runtime] backend` in config
4. `http`

```bash
orbit activity run path/to/agent.yaml --backend cli
```
