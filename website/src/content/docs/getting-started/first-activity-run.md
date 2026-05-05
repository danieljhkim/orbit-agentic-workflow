---
title: Activity Catalog
description: "List and inspect the schemaVersion 2 activities available to jobs."
sidebar:
  order: 4
---

## List Activities

Orbit ships default activities for job workflows. List them before wiring or inspecting a job.

```bash
orbit activity list
orbit activity list --json
```

Use `--ops` for the compact signal shape.

```bash
orbit activity list --ops
```

## Run a Job

Activities execute as job steps through `orbit job run` or workflow aliases under `orbit run`.
Use `orbit job list` to find runnable workflows.

```bash
orbit job list
orbit job run task_auto_pipeline
```

Pass input to the job when its activities expect it:

```bash
orbit job run task_auto_pipeline --input mode=local
```

## Choose a Backend

**v1 supports `backend: cli` only.** For `agent_loop` activities, backend resolution follows this order:

1. `--backend`
2. `ORBIT_BACKEND`
3. `[runtime] backend` in config
4. hard-coded fallback: **`http`**

Because the fallback is `http`, you must pin `cli` explicitly in v1 — either with `--backend cli`, `ORBIT_BACKEND=cli`, or `[runtime] backend = "cli"` in your config. Activities that declare `backend: cli` directly in YAML are unaffected.

```bash
orbit job run task_auto_pipeline --backend cli
```

`backend: http` is wired in code but is not part of the v1 release surface. Treat it as preview only and expect API churn until v2.
