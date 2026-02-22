# Orbit v2.1 Implementation Checklist (One Page)

**Goal:** Build a deterministic, CLI-first Orbit runtime with a single execution pipeline and auditable state changes.  
**Scope:** v2.1 foundational architecture + first vertical slice.  
**Out of Scope:** daemon runtime, TUI polish, crate extraction for scheduler/watcher.  
**Assumptions:** Rust stable toolchain; SQLite available; local development only.  
**Key Decision:** Jobs and watches stay in `orbit-core` as `orbit-core::job` and `orbit-core::watch`.

## Done Criteria

- [ ] Workspace layering compiles with strict downward dependencies.
- [ ] All command/job/watch execution flows through a single pipeline owned by `orbit-core`.
- [ ] **No direct persistence mutations** occur outside `orbit-core` runtime helpers (store writes are not called from CLI/tools directly).
- [ ] Every state mutation emits an `OrbitEvent` and persists an audit record in the same logical transaction boundary.
- [ ] Concurrency is deterministic: overlap protection exists for job runs and watch triggers via advisory locking + atomic transitions.
- [ ] First slice works end-to-end:
  - `orbit tool run fs.read --path plan_1.md`
  - flow: CLI -> Core -> Policy -> Tool -> Exec -> Event -> Audit -> Store

## Architectural Invariants (v2.1)

- **Single execution pipeline:** all entrypoints (CLI, job runner, watch runner) dispatch into the same `orbit-core` pipeline.
- **Auditability:** every mutation must produce `OrbitEvent` + audit record; bypassing this is a bug.
- **Strict layering:** crates only depend downward as documented; fail CI on forbidden edges.
- **No daemon in v2.1:** long-running behavior is explicit (`orbit watch run` foreground; jobs via `orbit job run` + cron).
- **No hidden concurrency:** no background threads that mutate state outside explicit triggers.
- **CLI stays thin:** CLI calls `orbit-core` only; it never touches store/tools/exec directly.

## Ordered Tasks

### 1. Workspace Skeleton and Boundaries

- [ ] Convert root crate into workspace in `/Users/daniel/repos/rust-projects/orbit-v2/Cargo.toml`.
- [ ] Create crates:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-cli`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-core`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-types`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-store`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-policy`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-exec`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-tools`
- [ ] Add dependency edges to enforce:
  - `orbit-cli -> orbit-core`
  - `orbit-core -> orbit-policy | orbit-exec | orbit-tools | orbit-store | orbit-types`
  - lower crates -> `orbit-types` only where shared contracts are needed.
- [ ] Verification:
  - `cargo check --workspace`

### 2. `orbit-types` Foundation Contracts

- [ ] Add core types and enums in `/Users/daniel/repos/rust-projects/orbit-v2/orbit-types/src/lib.rs`:
  - IDs, Task, Memo, Job, Watch, Audit, `OrbitEvent`, `ExecutionResult`, shared errors.
- [ ] Keep crate pure: no IO, no runtime behavior, no DB calls.
- [ ] Add unit tests for serialization/shape stability in `/Users/daniel/repos/rust-projects/orbit-v2/orbit-types/src/lib.rs`.
- [ ] Verification:
  - `cargo test -p orbit-types`

### 3. `orbit-store` Persistence

- [ ] Add SQLite connection/migrations in:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-store/src/lib.rs`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-store/migrations/`
- [ ] Create tables: `tasks`, `memos`, `jobs`, `watches`, `audits`.
- [ ] Implement CRUD + transactional helpers + **advisory locking primitives** (`try_lock(name)`, `unlock(name)` or equivalent) used by `orbit-core`.
- [ ] Add integration tests (temp DB) under:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-store/tests/`
- [ ] Verification:
  - `cargo test -p orbit-store`

### 4. `orbit-policy` Evaluation Engine

- [ ] Implement role/constraint model and `Allow | Deny` decision in:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-policy/src/lib.rs`
- [ ] Ensure pure evaluation only (no process spawning, no DB mutation).
- [ ] Add policy tests for default deny/allow and constrained inputs.
- [ ] **Policy default (v2.1 decision):** default to **ALLOW** for local single-user runtime; support explicit deny rules and test deny paths.
- [ ] Verification:
  - `cargo test -p orbit-policy`

### 5. `orbit-exec` Primitive Execution Layer

- [ ] Implement process runner, sandbox abstraction, timeout, output capture in:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-exec/src/lib.rs`
- [ ] Return structured execution output only.
- [ ] Add tests for timeout and output capture behavior.
- [ ] Verification:
  - `cargo test -p orbit-exec`

### 6. `orbit-tools` Tool Trait and Builtins

- [ ] Define `Tool` trait and registry in:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-tools/src/lib.rs`
- [ ] Implement initial builtins:
  - `fs.read`
  - `fs.write`
  - `proc.spawn`
  - `time.now`
- [ ] Wire tools to `orbit-exec` where needed.
- [ ] Add tool tests for schema + execution output shape.
- [ ] Verification:
  - `cargo test -p orbit-tools`

### 7. `orbit-core` Runtime Orchestration

- [ ] Add `OrbitContext` and unified pipeline in:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-core/src/lib.rs`
- [ ] Define `OrbitContext` shape explicitly (no partial contexts):
  - `store: orbit_store::Store`
  - `policy: orbit_policy::PolicyEngine`
  - `registry: orbit_tools::ToolRegistry`
- [ ] Define a clear mutation boundary helper (e.g., `with_mutation(|tx| ...)`) that guarantees: emit `OrbitEvent` + persist audit + commit.
- [ ] Add internal modules:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-core/src/job.rs`
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-core/src/watch.rs`
- [ ] Enforce flow:
  - command trigger -> policy -> tool/exec -> event emit -> audit persist.
- [ ] Add event bus and centralized audit emission hooks.
- [ ] Add execution tests for:
  - policy denied path (no side effects, audit/event recorded as denied)
  - successful tool path (event + audit persisted)
  - **mutation guard**: any write attempted outside runtime helper fails (or is unrepresentable)
  - job overlap protection: `orbit job run` cannot double-run the same due job
  - watch debounce behavior: burst events coalesce deterministically
- [ ] Verification:
  - `cargo test -p orbit-core`

### 8. `orbit-cli` Thin Command Layer

- [ ] Implement command parsing and dispatch in:
  - `/Users/daniel/repos/rust-projects/orbit-v2/orbit-cli/src/main.rs`
- [ ] Keep CLI thin: no business logic, no direct DB/tool calls.
- [ ] Enforce that CLI only depends on `orbit-core` APIs (no direct imports of `orbit-store`, `orbit-tools`, `orbit-exec`).
- [ ] Implement initial surfaces:
  - `orbit tool run`
  - `orbit task add`
  - `orbit task list`
  - `orbit audit list`
  - `orbit job run`
  - `orbit watch run`
- [ ] Verification:
  - `cargo run -p orbit-cli -- tool run fs.read --path /Users/daniel/repos/rust-projects/orbit-v2/plan_1.md`

## Final Verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo run -p orbit-cli -- tool run fs.read --path /Users/daniel/repos/rust-projects/orbit-v2/plan_1.md`

## Risks and Mitigations

- [ ] **Risk:** crate layering drift over time.  
  **Mitigation:** keep dependencies explicit and fail CI on forbidden edges.
- [ ] **Risk:** audit/event bypass in ad-hoc code paths.  
  **Mitigation:** funnel all state transitions through runtime helpers.
- [ ] **Risk:** direct store writes bypassing runtime mutation boundary.  
  **Mitigation:** make store write APIs internal to `orbit-core` usage patterns; prefer compile-time restriction by not exposing write fns to CLI/tools.
- [ ] **Risk:** watch/job overlap causing nondeterminism.  
  **Mitigation:** locking + atomic transitions + deterministic conflict policy.

## Open Questions

- [ ] Which SQLite crate/abstraction to standardize on across store and tests? (prefer `rusqlite` for v2.1; revisit if async required later)
- [ ] Should `watch run` skip overlapping events or queue one pending run by default? (recommend: debounce + queue-1)
