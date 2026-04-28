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

1. command flag (`--backend`)
2. `ORBIT_BACKEND`
3. `[runtime] backend`
4. hard-coded fallback

**v1 release scope.** v1 supports `backend: cli` only. Pin it explicitly so the resolution is deterministic regardless of build-internal defaults:

```toml
# orbit.toml
[runtime]
backend = "cli"
```

…or via environment for one-off invocations:

```bash
ORBIT_BACKEND=cli orbit run job task_auto_pipeline
```

Accepted backend values:

| Value | v1 status |
|-------|-----------|
| `cli` | Supported. The v1 release path. |
| `http` | Preview / not in v1 release surface. Wired in code for v2; do not depend on its behavior in v1. |
| `auto` | Resolves to a concrete backend at load time. Always pin `cli` explicitly in v1 instead of relying on `auto`. |

## Workspace State

Workspace-local state lives under `.orbit/` in the repository. Global state is initialized with `orbit init`, usually under `~/.orbit/`.
