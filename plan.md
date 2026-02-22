orbit/
├── Cargo.toml (workspace)
├── orbit-cli/        ← binary only
├── orbit-core/       ← domain runtime
├── orbit-policy/
├── orbit-store/
├── orbit-tools/
├── orbit-exec/
└── orbit-types/

orbit-core/
├── lib.rs
├── runtime/
├── command/
├── registry/
└── context.rs

```rust
pub struct OrbitContext {
    pub store: Store,
    pub policy: PolicyEngine,
    pub registry: ToolRegistry,
}
```

orbit-tools/
├── lib.rs
├── builtin/
│   ├── fs/
│   ├── proc/
│   ├── time/
│   └── net/
└── registry.rs

```rust
trait Tool {
    fn schema(&self) -> ToolSchema;
    fn execute(&self, ctx: &Context, input: Value) -> Result<Value>;
}

```

orbit-exec/
├── runner.rs
├── sandbox.rs
├── timeout.rs
└── process.rs


orbit-policy/
├── engine.rs
├── role.rs
├── constraint.rs
└── evaluator.rs

dependency direction:
```
orbit-cli
     ↓
orbit-core
     ↓
(policy | exec | tools | store)
     ↓
orbit-types
```

---

### Crate Boundary Decision (v2.1)

**Decision:** `orbit-watch` and `orbit-job` are NOT separate crates in v2.1.

Jobs and watches are implemented as modules inside `orbit-core`:

- `orbit-core::job`
- `orbit-core::watch`

Rationale:
- Jobs and watches are triggers into the unified execution pipeline, not independent subsystems.
- Avoid premature public APIs while trigger semantics (locking, retries, debounce, audit events) stabilize.
- Reduce crate proliferation and dependency edges during early architecture evolution.

Deferred extraction (v2.2+):
Split into dedicated crates only if one of the following becomes necessary:
- Feature-gating (e.g., build without `notify`)
- Independent reuse in other binaries
- Divergent runtime models (daemon vs foreground)
- Significant code growth and stabilization

If extracted later, prefer names:
- `orbit-scheduler` (jobs)
- `orbit-watcher` (filesystem watches)

---

### Orbit Internal Architecture v2
```
                User / Agent
                      │
                      ▼
               orbit-cli (UX)
                      │
                      ▼
              Command Dispatcher
                      │
                      ▼
                Orbit Runtime
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
   Policy Engine   Tool Registry   Scheduler
        │             │             │
        ▼             ▼             ▼
   Execution Engine ─────────► State Store
        │
        ▼
   OS / Filesystem / Network
```


#### 9. Unified Event Bus (Major v2 Addition)
- publish events - ToolExecuted, etc

---

### 10. Job Execution Model (v2 Clarification)

**Decision:** No daemon (initially). Jobs are executed via explicit trigger.

Command surface:
```
orbit job run
```

Execution semantics:
- Loads due jobs (`next_run_at <= now`)
- Acquires global execution lock
- Marks job as `running`
- Executes associated workflow/tool
- Emits structured audit event
- Updates `last_run_at` + recomputes `next_run_at`
- Releases lock

Concurrency protection:
- SQLite-based advisory lock OR file lock
- Atomic state transition: `scheduled -> running -> complete`
- Prevent overlapping execution

Rationale:
- Keeps system synchronous
- Avoids daemon lifecycle complexity
- Unix-native (cron integration)

Future expansion (optional):
- `orbit daemon` unifying jobs + watch if required

---

### 11. Watch Execution Model (v2 Clarification)

**Decision:** Foreground long-running process.

Command surface:
```
orbit watch run
```

Execution semantics:
- Uses `notify` crate
- Loads watch definitions from store
- Blocks in foreground
- Debounces filesystem events (configurable window)
- Matches event → watch rule
- Executes workflow/tool
- Emits audit event

Concurrency model:
- Per-watch execution guard
- Prevent overlapping runs
- Optional queue or skip mode

Rationale:
- Avoids daemon complexity
- Debuggable
- Deterministic
- User controls lifecycle (tmux/systemd/launchd)

---

### 12. Execution Engine Contract

All executions (CLI / job / watch) must flow through a single execution pipeline:

```
Command → Policy Check → Sandbox → Tool Execute → Emit Event → Persist Audit
```

Execution guarantees:
- Policy enforced before tool execution
- Timeout enforcement
- Structured output
- Deterministic state mutation

---

### 13. Audit Emission Rules

Every state mutation MUST emit an audit event.

Examples:
- ToolExecuted
- JobStarted
- JobCompleted
- WatchTriggered
- PolicyDenied
- WorkflowTransition

Audit emission is centralized in `orbit-core` runtime layer.

No direct DB mutation allowed without emitting an event.

---

### 14. Unified Event Bus (Expanded)

Event bus responsibilities:
- Internal publish/subscribe mechanism
- Decouple execution from side effects
- Allow future features (metrics, tracing, AI hooks)

Event example:
```rust
pub enum OrbitEvent {
    ToolExecuted { name: String },
    JobStarted { id: String },
    WatchTriggered { path: String },
}
```

Event flow:
```
Execution Engine
      ↓
Event Bus
      ↓
Audit Sink / Metrics / Observability
```

---

### 15. Locking & Concurrency Strategy

- Global execution lock for `job run`
- Per-watch execution guard
- SQLite transactional state transitions
- No implicit concurrent mutation

Design principle:
> Orbit remains deterministic under concurrent triggers.

---

### 16. Profile Strategy (Deferred)

Current model: Single runtime root.

Future extension:
```
~/.orbit/profiles/<name>/
```

All path resolution derived from runtime root abstraction.

---

### 17. Dependency Philosophy

Strict layering:

```
orbit-cli → orbit-core
orbit-core → (policy | exec | tools | store)
(policy | exec | tools | store) → orbit-types
```

No upward dependencies.
No circular imports.
No execution outside runtime pipeline.

---

### 18. Deterministic Runtime Principle

Orbit is:
- CLI-first
- Synchronous by default
- Event-driven internally
- Daemon-optional
- Explicit execution only

No hidden background threads in v2.

---