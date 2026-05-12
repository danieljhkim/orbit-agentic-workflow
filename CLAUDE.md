# CLAUDE.md

Project instructions for agents working on Orbit.

## Rules

- **Don't commit** until the Orbit task has been explicitly approved by the human.
- **Don't invent task IDs** — get them from `orbit.task.add`. Don't edit task files directly — use `orbit.task.update`.
- **Don't add cross-crate dependencies** without checking [`ARCHITECTURE.md`](ARCHITECTURE.md). If a new edge is genuinely needed, file a task and an ADR before adding it.
- **Use subagents** for large tasks to keep your context window clean.

## Build / Lint

`make build`, `make fmt`, `make ci` — all must pass before a task moves to `review`. `make ci` runs `cargo clippy --workspace --all-targets -- -D warnings`; mechanical rules live in `[workspace.lints]` in the root `Cargo.toml`, not in this file. Add to the lint table rather than to prose when something can be checked automatically.

## Architecture

Crate layering, per-crate responsibilities, and scoping rules live in [`ARCHITECTURE.md`](ARCHITECTURE.md). Read it before adding a new crate, a new dependency edge, or a new persisted artifact.

Reusable codebase-specific patterns (Command, RAII guard, newtype, crate-boundary error translation) live in [`docs/design-patterns/`](docs/design-patterns/). When you reach for one of those shapes, copy from the documented reference instead of inventing a new one.

## Design Docs

- **Layout.** Feature design docs live under `docs/design/<feature>/` and follow [`CONVENTIONS.md`](docs/design/CONVENTIONS.md) (folder layout, required sections, ADR format, glossary shape).
- **Same-PR updates.** Change the doc in the same PR as the code: flip affected ADR statuses (`Proposed → Accepted` with task ID), bump `**Last updated:**`, add a new ADR for any non-obvious decision the change embodies. Stale docs are a review blocker.
- **Decay check.** `make check-design-docs` flags `docs/design/*` docs whose `**Last updated:**` date precedes the last commit on any `crates/...rs` file they reference. Run it before review; fix flagged docs or update their `Last updated`.

## Rust Practices

Judgment calls that the workspace lint table can't catch:

- **Errors:** propagate via `OrbitError` at crate boundaries; reach for typed `thiserror` variants over ad-hoc strings. Don't `unwrap` / `expect` in non-test code unless the invariant is local and stated in the message. See [`docs/design-patterns/error_translation.md`](docs/design-patterns/error_translation.md) for the boundary-translator shape.
- **Logging vs user output:** `tracing` for diagnostics; `println!` / `eprintln!` only for genuine CLI user-facing output in `orbit-cli` (and example binaries). Prefer structured fields (`tracing::info!(run_id, ...)`) over string interpolation. The default subscriber handles redaction — don't reach around it.
- **Visibility:** default to `pub(crate)`; reserve `pub` for items in the crate's documented public surface (see `ARCHITECTURE.md`). Re-export at the crate root only for types genuinely part of the API.
- **Async locking:** never hold a `std::sync::Mutex` / `RwLock` guard across `.await`. Scope the lock to a block, or use `tokio::sync` primitives when state is genuinely cross-task. Bounded channels by default.
- **Tests:** in-file unit tests under `#[cfg(test)] mod tests`; integration tests under `tests/`. Match nearby scaffolding (e.g. sibling `*_tests.rs` files in `orbit-engine::activity_job::job_executor`). Don't introduce a new test harness when an existing one fits.

## Commits & Authorship

- Use the agent commit identity (e.g. `codex`, `claude`) as author/committer.
- Include the Orbit task ID in commit messages when applicable (e.g. `[ORB-00042]`). Task IDs are allocation-authority search keys (`git log --grep '[ORB-00042]'`); when a task has a linked `external_ref`, include that tag too (`[ORB-00042] [ENG-1234] ...`) — cross-engineer reviewers resolve the external tag, not the Orbit one.
- Use your model name (e.g. `claude-opus-4-7`, `gpt-5.5`, `gemini-3.1-pro`) when authoring tasks or docs. Cite relevant task IDs in any doc you write.

## Orbit Workflow

For any Orbit lifecycle work (creating tasks, executing, reviewing, raising PRs), invoke the relevant `orbit-*` skill. The `orbit` skill is the entry point and router. Task authoring quality standards live in `orbit-create-task`.

Scoreboards live at `.orbit/state/scoreboard/` (e.g. `duel_plan.json` — planning-duel run results).
