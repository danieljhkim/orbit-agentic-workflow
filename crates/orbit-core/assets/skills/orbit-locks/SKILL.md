---
name: orbit-locks
description: Use when reserving or releasing Orbit file locks for ad-hoc code modification outside workflow-dispatched task runs.
---

# Orbit Locks

Use `orbit.task.locks.reserve` before modifying files outside a workflow-held task reservation, then release the reservation with `orbit.task.locks.release` when modification is complete.

## When To Reserve

Reserve before editing when you are doing ad-hoc agent work and no `task_gate_pipeline` reservation already covers the file. Pass canonical file or directory selectors through the `files` shape:

```json
{
  "files": ["file:src/foo.rs", "dir:src/auth"],
  "model": "<model_name>"
}
```

Only use `file:` and `dir:` selectors. Locking is file/directory scoped; `symbol:` selectors are not accepted.

## Conflict Handling

If `orbit.task.locks.reserve` returns `"reserved": false`, sleep and retry instead of editing through the conflict.

Use this backoff:

1. Start with 5 seconds.
2. Double each retry up to a maximum delay of 30 seconds.
3. Stop after 10 retries, about 2 minutes total wait.

After the final retry, surface the conflict to the operator with the returned conflict list. Do not spin forever.

## Release

After modification, call `orbit.task.locks.release` with the `reservation_id` returned by `orbit.task.locks.reserve`:

```json
{
  "reservation_id": "reservation-...",
  "model": "<model_name>"
}
```

Release even when validation fails, unless keeping the lock is necessary to prevent another agent from building on a known-bad partial edit.

## TTL

The default reservation TTL is 1800 seconds. If work may take longer, pass `ttl_seconds` on the reserve call, up to 7200:

```json
{
  "files": ["file:src/foo.rs"],
  "ttl_seconds": 7200,
  "model": "<model_name>"
}
```

## Workflow Runs

Agents running inside a `task_gate_pipeline` dispatched workflow should not call `orbit.task.locks.reserve`. The gate already holds the lock under the run's `owner_run_id`; duplicate reservations add noise and can create self-conflicts.
