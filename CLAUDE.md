# CLAUDE.md

Project instructions for agents working on Orbit.

## Project Don'ts

- Don't commit until the Orbit task has been explicitly approved by the human.
- Don't modify files outside the task's scope or `context_files` list.
- Don't invent task IDs — get them from `orbit.task.add`.
- Don't edit task files directly — use `orbit.task.update`.
- Don't add cross-crate dependencies without checking the architecture diagram below.
- When adding a store module, match the atomic-write pattern of `orbit/orbit-store/src/file/pr_scoreboard.rs` — do not invent a new one.
- When you hit friction, ambiguity, naming drift, or duplicated sources of truth: file a friction task via the `orbit-track-issues` skill instead of working around it.

## Project Do's

- Use subagents to support you through large tasks and keep your context window clean.
- Use terse/succinct prose in all agent-written text: tasks, docs, comments, commit messages.
- If a task spec contradicts the code, acceptance criteria are ambiguous, or you see a simpler approach: stop and flag it before implementing.
- Keep lib.rs and mod.rs thin but intentional

## Build / Test / Lint

- Build: `make build`
- Test:  `make test`
- Lint:  `make clippy`
- Fmt:   `make fmt`
- CI:    `make ci` (runs fmt + clippy + test)

All must pass before a task moves to `review`.

## Crate Architecture

```
orbit-types → orbit-policy, orbit-exec → orbit-tools → orbit-store, orbit-agent → orbit-engine → orbit-core → orbit-cli
```

- **orbit-types**: leaf — no internal deps. Shared types, `OrbitError`, ID generation.
- **orbit-store**: layered store pattern (YAML + SQLite). Match existing modules when adding new ones.
- **orbit-agent**: `AgentRuntime` trait. New agent families go here, not elsewhere.
- **orbit-engine**: activity/job execution, template rendering, retry logic.
- **orbit-core**: runtime bootstrap, config layering, command dispatch, default asset seeding.
- **orbit-cli**: clap-based CLI entry point.

## Scoping Rules

| Artifact        | Strategy           | Rationale                                        |
|-----------------|--------------------|--------------------------------------------------|
| Tasks           | WorkspaceOnly      | Per-repo backlog, no cross-project leaking       |
| Activities/Jobs | MergeByKey         | Global defaults + workspace overrides            |
| Job Runs        | WorkspaceOnly      | Execution artifacts are workspace-local          |
| Skills          | WorkspaceReplaces  | Workspace has full control over available skills |
| Audit           | GlobalOnly         | Single authoritative event trail                 |

## Orbit Workflow

For any Orbit lifecycle work (creating tasks, executing, reviewing, raising PRs), invoke the relevant `orbit-*` skill. The `orbit` skill is the entry point and router.

## Task Authoring Quality

Follow the `## Task Quality Standards` section in `orbit-create-task` skill: deterministic mock-based testing, explicit observable definitions for summary fields (`purpose`, etc.), and testability-preserving implementation patterns.

## Agent Identity

**Signature** (used in PR bodies, comments, commit messages, review summaries):

> *authored by: claude / opus*

**Commits**:

- Use the agent commit identity (e.g. `claude`) as author/committer when the agent made the change.
- Include the task ID in the commit message when the commit is associated with an Orbit task (e.g. `[T20260320-001234]`).

## Scoreboards

- `.orbit/scoreboard/pr.json` — PR merge rates, revision counts, review comment validity per agent/model.
- `.orbit/scoreboard/friction_bounty.json` — friction reports (issues-reported, issues-accepted, issues-rejected) per agent/model. Rejected reports count against the reporter; quality over quantity.
