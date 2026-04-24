---
title: Configuration
description: "Runtime configuration surfaces and backend precedence."
sidebar:
  order: 5
---

## Root Override

Most commands accept the global `--root` option to override the Orbit root directory.

```bash
orbit --root /path/to/orbit-root task list
```

## Backend Precedence

For `agent_loop` execution, backend selection resolves once before dispatch.

1. command flag
2. `ORBIT_BACKEND`
3. `[runtime] backend`
4. `http`

```bash
ORBIT_BACKEND=cli orbit activity run path/to/agent.yaml
orbit activity run path/to/agent.yaml --backend http
```

Accepted backend values:

- `http`
- `cli`
- `auto`

## Workspace State

Workspace-local state lives under `.orbit/` in the repository. Global state is initialized with `orbit init`, usually under `~/.orbit/`.
