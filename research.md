# Orbit v2.1 Plan Summary (In My Own Words)

## What Orbit v2.1 is trying to become

Orbit v2.1 is a CLI-first runtime for running tools and automations in a controlled, deterministic way.  
The design emphasizes explicit execution, strict architecture boundaries, and observable state changes.

At a high level, Orbit should feel like:
- A thin CLI interface on top
- A single runtime orchestrator in the middle
- Focused subsystems beneath it (policy, execution, tools, storage, shared types)

## Core architecture intent

The workspace is intentionally layered so dependencies only flow downward:

`orbit-cli -> orbit-core -> (orbit-policy | orbit-exec | orbit-tools | orbit-store) -> orbit-types`

This dependency direction is a hard rule to avoid circular design and to keep long-term maintenance manageable.

## Crate boundary decision for v2.1

`orbit-watch` and `orbit-job` are **not** separate crates in v2.1.

Jobs and watches live inside `orbit-core` as:
- `orbit-core::job`
- `orbit-core::watch`

Reasoning:
- Jobs and watches are trigger mechanisms into the same execution pipeline.
- Separate crates would introduce premature APIs and extra dependency complexity.
- Extraction is deferred until there is a clear need (feature gating, reuse, runtime divergence, or major growth).

If split later (v2.2+), preferred names are:
- `orbit-scheduler` (jobs)
- `orbit-watcher` (filesystem watches)

## Runtime contract (single execution pipeline)

All execution modes (direct CLI commands, scheduled jobs, and file watches) must run through one contract:

`Command -> Policy Check -> Sandbox/Execution -> Event Emission -> Audit Persistence`

The plan treats this as non-negotiable so behavior remains consistent and auditable across features.

## Determinism and observability principles

Orbit v2.1 is:
- Synchronous by default
- Event-driven internally
- Explicitly triggered (no hidden background behavior)
- Daemon-optional (not required for v2.1)

Every state mutation must emit an audit/event record.  
No direct state mutation should bypass audit emission.

## Jobs and watches execution model

### Jobs
- Triggered by explicit command (`orbit job run`)
- Pull due jobs (`next_run_at <= now`)
- Use lock protection to prevent overlap
- Move through atomic states (scheduled -> running -> complete)
- Emit structured audit events

### Watches
- Run in foreground (`orbit watch run`)
- Load watch definitions and block as a long-running process
- Debounce filesystem events
- Apply per-watch overlap protection
- Emit audit events for triggers/executions

Both are intentionally modeled as deterministic triggers, not daemon-managed background subsystems.

## Build order (phased plan)

1. **Phase 0**: Workspace skeleton and dependency boundaries
2. **Phase 1**: `orbit-types` (shared domain contracts only)
3. **Phase 2**: `orbit-store` (SQLite, migrations, CRUD, locking primitives)
4. **Phase 3**: `orbit-policy` (allow/deny evaluation logic)
5. **Phase 4**: `orbit-exec` (sandbox/process/timeout execution primitives)
6. **Phase 5**: `orbit-tools` (Tool trait, registry, built-in tools)
7. **Phase 6**: `orbit-core` (runtime orchestration, events, audits, job/watch modules)
8. **Phase 7**: `orbit-cli` (parsing/dispatch/output only)
9. **Phase 8**: Vertical features (`tool run`, task ops, audit list, job run, watch run, optional TUI)

## First milestone

The first end-to-end proof is a minimal vertical slice:

`orbit tool run fs.read --path plan_1.md`

Expected path:

`CLI -> Core -> Policy -> Tool -> Exec -> Event -> Audit -> Store`

If this works, the architecture is validated before expanding features.
