# CLAUDE.md

Project instructions for agents working on Orbit.

## Rules

- **Don't commit** until the Orbit task has been explicitly approved by the human.
- **Don't invent task IDs** — get them from `orbit.task.add`. Don't edit task files directly — use `orbit.task.update`.
- **Don't add cross-crate dependencies** without checking the architecture diagram below.
- **Use subagents** for large tasks to keep your context window clean.
- **Hit friction?** File it via the `orbit-track-issues` skill (records append-only under `.orbit/frictions/`). Use for Orbit tooling / skill / seeded-instruction problems only — not for user-requested work or generic bug tracking.

## Spotted Issues → Task + FOLLOWUP

When you notice something out-of-scope for the current task — convention break, failing test, perf regression, security hole, modularization smell, dead code — convert it to a durable artifact instead of reporting it back in chat. Bare `TODO` / `FIXME` comments are not used in this codebase; `FOLLOWUP(<task_id>)` is the only sanctioned marker.

- **Threshold.** File only when the fix is *out-of-scope for the current task* AND *non-trivial*. Trivial fixes (typo, one-line cleanup) → fix inline. Vague observations ("could be cleaner") → drop, don't file.
- **Marker format.** `// FOLLOWUP(T20260510-25): <one-line description>` — grep-able by task ID; the task body carries full context. Comment syntax matches the surrounding language (`#` in YAML / Python, `//` in Rust).
- **Where the marker lives.** Only in files the current task is *already* modifying. If the issue is in a different file, file the task with a file/line reference in the body — don't open an unrelated file just to mark it, that bleeds scope into the current PR.
- **Resolution.** The implementing agent must `git grep "FOLLOWUP(<task_id>)"` before marking the task done and delete every match as part of the fix. A stale `FOLLOWUP(T...)` referencing a `done` / `archived` task is a review blocker.

## Build / Lint

`make build`, `make fmt`, `make ci` — all must pass before a task moves to `review`.

## Design Docs

Feature design docs live under `docs/design/<feature>/` and follow [`CONVENTIONS.md`](docs/design/CONVENTIONS.md) (folder layout, required sections, ADR format, glossary shape).

**Read before changing a feature.** Start at `1_overview.md` for what/why, then `2_design.md` for current implementation. `4_decisions.md` is the ADR log; `specs/` carries mechanism-level contracts. Cite design-doc sections in PR descriptions when behavior is non-obvious.

**Update in the same PR as the implementation change.** Flip affected ADR statuses (`Proposed → Accepted` with task ID), bump `Last updated`, add a new ADR for any non-obvious decision the change embodies. Stale docs are a review blocker.

**Feature Lead responsibility.** The lead is accountable for their feature's design-doc hygiene and create tasks to ensure the docs are maintained, as well as proactively identifying design-related concerns via filing friction tasks.

| Feature | Folder | Lead |
|---------|--------|------|
| Knowledge graph | `knowledge-graph/` | `claude` |
| Policy & Sandboxing | `policy-sandbox/` | `claude` |
| Project Learnings | `project-learnings/` | `claude` |
| Task Lineage | `task-lineage/` | `claude` |
| Semantic Search | `semantic-search/` | `claude` |
| Task Sync | `task-sync/` | `claude` |
| Activity / Job | `activity-job/` | `codex` |
| Auditability | `auditability/` | `codex` |
| Groundhog | `groundhog/` | `codex` |
| User Interface | `user-interface/` | `gemini` |

## Crate Architecture

```
orbit-common → orbit-policy, orbit-exec, orbit-knowledge, orbit-embed → orbit-tools → orbit-agent → orbit-engine → orbit-core → orbit-cli
            ↘ orbit-store ──────────────────────────────────────────────────────────────────────────↗            ↗
            ↘ orbit-embed-companion (depends on orbit-embed; not linked into the default `orbit` binary)
            ↘ orbit-mcp ─────────────────────────────────────────────────────────────────────────────────────────↗
```

- **orbit-common**: leaf — no internal deps. `types::` owns shared domain types, `OrbitError`, ID generation, and activity/job schemas; `utility::` owns generic helpers like fs, redaction, logging, and blob storage.
- **orbit-policy**: filesystem-scoping policy engine. Owns `FsProfile` resolution and `denyRead` / `denyModify` evaluation. Depends only on `orbit-common`.
- **orbit-exec**: process / sandbox / supervision primitives for shell-command execution under an `FsProfile`. Depends only on `orbit-common`.
- **orbit-embed**: semantic-embedding feature crate. Owns the `Embedder` trait, JSON-Lines RPC types, `SubprocessEmbedder`, `NoopEmbedder`, the workspace-local vector store (`vector::VectorStore` with its own `rusqlite::Connection`, WAL + busy_timeout pragmas, idempotent `embeddings` / `tasks_fts` schema, `EmbedWorker`, paragraph chunker, BLAKE3 dedup, cosine helper), and the install/uninstall/reindex/stats `commands::*` surface. Depends only on `orbit-common`; does not depend on `orbit-store` or fastembed-rs.
- **orbit-embed-companion**: separately installed embedding companion binary. Depends on `orbit-embed` and fastembed-rs; not linked into the default `orbit` CLI binary.
- **orbit-knowledge**: knowledge/graph parsing and storage helpers. Multi-language source parsing (Rust, Go, Java, JavaScript/TypeScript, Python). Depends on `orbit-common`; consumed by `orbit-tools`, which exposes graph tool and CLI-use-case facades upstream.
- **orbit-store**: layered store pattern (YAML + SQLite). Match existing modules when adding new ones. Depends only on `orbit-common`; the semantic vector schema is owned by `orbit-embed::vector` (not `orbit-store`).
- **orbit-tools**: tool registry plus built-in graph, fs, and policy-aware exec tools. Depends on `orbit-common`, `orbit-exec`, `orbit-knowledge`, `orbit-policy`.
- **orbit-mcp**: Model Context Protocol adapter using `rmcp`. Depends only on `orbit-common`; consumed by `orbit-cli` via `orbit mcp serve`.
- **orbit-agent**: per-provider `AgentRuntime` implementations under `providers/<name>/<name>_runtime.rs` (claude, codex, gemini, openai_compat, anthropic, ollama, mock_agent). Implements `backend: cli`. Also hosts HTTP `LoopTransport` primitives.
- **orbit-engine**: activity/job execution, template rendering, retry logic. Owns the `backend: cli` subprocess runner (`activity_job::cli_runner`), which references `orbit-agent::{Agent, AgentConfig}` directly so orbit-core stays clean of orbit-agent types.
- **orbit-core**: runtime bootstrap, config layering, command dispatch, default asset seeding. Surfaces the `OrbitRuntime` API used by `orbit-cli`; does NOT depend on `orbit-agent`.
- **orbit-cli**: clap-based CLI entry point.

## Scoping Rules

| Artifact        | Strategy           | Rationale                                        |
|-----------------|--------------------|--------------------------------------------------|
| Tasks           | WorkspaceOnly      | Per-repo backlog, no cross-project leaking       |
| Activities/Jobs | MergeByKey         | Global defaults + workspace overrides            |
| Policies        | MergeByKey         | Workspace overrides profiles by name; global `denyRead` / `denyModify` rules accumulate |
| Job Runs        | WorkspaceOnly      | Execution artifacts are workspace-local          |
| Skills          | MergeByKey         | Global defaults in `~/.orbit/skills`; workspace overrides by skill name |
| Command Audit   | GlobalOnly         | Single authoritative SQLite event trail          |
| Semantic Index  | WorkspaceOnly      | Task-derived embeddings stay with the workspace  |
| Run Traces      | WorkspaceOnly      | Per-repo activity/job JSONL and blob artifacts   |

## Maintainer Conventions

- Order files for the next maintainer's first read: lead with the module's primary concept or entry point, then move into supporting detail.
- Data-heavy modules usually start with their core structs/enums; orchestration modules usually start with the main public function.

**Rust practices:**

- **Errors:** propagate via `OrbitError` at crate boundaries; reach for typed `thiserror` variants over ad-hoc strings. `?` over manual `match`. Don't `unwrap` / `expect` in non-test code unless the invariant is local and stated in the message.
- **Logging:** use `tracing` (never `println!` / `eprintln!` / `log`). The default subscriber handles redaction — don't reach around it. Prefer structured fields (`tracing::info!(run_id, ...)`) over string interpolation.
- **Visibility:** default to `pub(crate)`; reserve `pub` for items in the crate's documented public surface (see the architecture diagram). Re-export at the crate root only for types that are genuinely part of the API.
- **Async locking:** never hold a `Mutex` / `RwLock` guard across `.await`. Scope the lock to a block, or use `tokio::sync` primitives when state is genuinely cross-task. Bounded channels by default.
- **Tests:** in-file unit tests under `#[cfg(test)] mod tests`; integration tests under `tests/`. Match nearby scaffolding (e.g. sibling `*_tests.rs` files in `orbit-engine::activity_job::job_executor`). Don't introduce a new test harness when an existing one fits.
- **Comments & docs:** `///` doc-comment public items. Explain *why* in code comments, not *what* — well-named identifiers carry the *what*. Remove commented-out code instead of fencing it.

## Commits & Authorship

- Use the agent commit identity (e.g. `codex`, `claude`) as author/committer.
- Include the Orbit task ID in commit messages when applicable (e.g. `[T20260320-001234]`). Task IDs are workspace-local search keys (`git log --grep '[T...]'`); when a task has a linked `external_ref`, include that tag too (`[T20260506-11] [ENG-1234] ...`) — cross-engineer reviewers resolve the external tag, not the Orbit one.
- Use your model name (e.g. `claude-opus-4-7`, `gpt-5.5`, `gemini-3.1-pro`) when authoring tasks or docs. Cite relevant task IDs in any doc you write.

## Orbit Workflow

For any Orbit lifecycle work (creating tasks, executing, reviewing, raising PRs), invoke the relevant `orbit-*` skill. The `orbit` skill is the entry point and router. Task authoring quality standards live in `orbit-create-task`.

Scoreboards live at `.orbit/state/scoreboard/` (e.g. `duel_plan.json` — planning-duel run results).
