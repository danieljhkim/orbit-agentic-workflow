

# Orbit v2 – Build Order & Architecture Plan

This document defines the exact crate build order and architectural layering for Orbit v2.

The goal is to enforce strict dependency direction, deterministic execution, and long-term maintainability from day one.

---

# Workspace Layout

```
orbit/
├── Cargo.toml (workspace)
├── orbit-cli/
├── orbit-core/
├── orbit-types/
├── orbit-store/
├── orbit-policy/
├── orbit-exec/
├── orbit-tools/
```

Dependency direction must always flow downward:

```
orbit-cli → orbit-core
orbit-core → orbit-policy | orbit-exec | orbit-tools | orbit-store
orbit-policy | orbit-exec | orbit-tools | orbit-store → orbit-types
```

No circular dependencies.
No upward references.

---

# Phase 0 – Workspace Skeleton

Create workspace structure and enforce dependency graph.

No logic yet.

Objective:
- Compiler enforces architecture boundaries from the beginning.

---

# Phase 1 – orbit-types (Foundation)

Define all shared domain contracts:

- ID types
- Task
- Memo
- Job
- Watch
- Audit
- OrbitEvent enum
- ExecutionResult
- Shared error types

Rules:
- No database access
- No filesystem access
- No runtime logic
- Pure data contracts only

This crate becomes the stable foundation layer.

---

# Phase 2 – orbit-store (Persistence Layer)

Responsibilities:

- SQLite connection management
- Migrations
- Table definitions:
  - tasks
  - memos
  - jobs
  - watches
  - audits
- CRUD operations
- Transaction helpers
- Locking primitives

Rules:
- No execution logic
- No policy enforcement
- No CLI interaction

Store only persists and retrieves state.

---

# Phase 3 – orbit-policy (Execution Gatekeeper)

Responsibilities:

- Role model
- Policy evaluation engine
- Constraint system
- PolicyDecision enum

Interface:

```
Context + ToolSchema + Input
→ Allow / Deny
```

Rules:
- No DB mutation
- No process spawning
- Pure evaluation logic

---

# Phase 4 – orbit-exec (Execution Engine Primitive)

Responsibilities:

- Process spawning
- Sandbox abstraction
- Timeout enforcement
- Output capture
- Structured execution results

Rules:
- No policy logic
- No DB access
- No CLI logic

This is a safe execution primitive layer.

---

# Phase 5 – orbit-tools (Builtin Tool Layer)

Responsibilities:

- Tool trait definition
- Tool registry
- Builtin tools (examples):
  - fs.read
  - fs.write
  - proc.spawn
  - time.now

Each tool:
- Implements Tool trait
- Uses orbit-exec when needed
- Returns structured output

Rules:
- No direct CLI access
- No DB mutation
- Tools are stateless execution units

---

# Phase 6 – orbit-core (Runtime Orchestrator)

This is the brain of Orbit.

Core responsibilities:

- OrbitContext struct
- Unified execution pipeline
- Event bus
- Audit emission
- Job execution logic
- Watch execution logic
- Workflow execution logic

Unified execution pipeline:

```
Command
  → Policy Check
  → Tool Execute
  → Emit Event
  → Persist Audit
```

All execution flows must pass through this pipeline.

Core depends on:
- orbit-types
- orbit-store
- orbit-policy
- orbit-exec
- orbit-tools

Nothing depends on orbit-core except CLI.

---

# Phase 7 – orbit-cli (Thin Interface Layer)

Responsibilities:

- Parse CLI commands
- Initialize OrbitContext
- Dispatch to orbit-core
- Print structured output
- Handle exit codes

Rules:
- No business logic
- No DB access
- No tool execution directly

CLI must remain thin.

---

# Phase 8 – Vertical Feature Expansion

After the runtime skeleton works end-to-end, implement features in this order:

1. `orbit tool run`
2. `orbit task add/list`
3. `orbit audit list`
4. `orbit job run`
5. `orbit watch run`
6. TUI (optional, separate crate later)

Each feature must pass through the full execution pipeline.

---

# Deterministic Runtime Principle

Orbit v2 is:

- CLI-first
- Synchronous by default
- Event-driven internally
- Explicit execution only
- Daemon-optional (not required for v2)

No hidden background threads.
No implicit state mutation.

All state transitions must emit audit events.

---

# First Milestone

Minimal working vertical slice:

```
orbit tool run fs.read --path plan_1.md
```

Flow:

CLI → Core → Policy → Tool → Exec → Event → Audit → Store

If this works end-to-end, the architecture is correct.

---

End of document.