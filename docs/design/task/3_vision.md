# Task — Vision

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-05-11

This document captures open questions for the task artifact reset, prior work that should inform the design, what may be distinctive about Orbit's task model, and references for future implementers. The near-term design is in [2_design.md](./2_design.md); this file is deliberately forward-looking.

---

## 1. Open Questions

### 1.1 How global should global IDs be?

`ORB-A0001` can mean at least three things:

1. Unique on one machine across all local workspaces.
2. Unique in one synced repository registry.
3. Unique across hosted Orbit Team.

The v2 design should pick the narrowest authority that serves the product surface being implemented. For OSS local-first, a machine-global allocator is enough. For task sync, repository-registry-global is enough. Hosted Team may later introduce org-global or tenant-global allocation.

### 1.2 Does the allocation series encode anything?

The `A` in `ORB-A0001` could remain a simple series marker, or it could encode allocator generation, tenancy, or artifact type. Encoding meaning makes debugging easier but ossifies the format. The current recommendation is to keep it as an allocation series only.

### 1.3 How structured should acceptance criteria become?

Markdown checkboxes are pleasant and reviewable. Structured checks are better for automation. Three candidate paths:

- Keep `acceptance.md` as source of truth and parse checkboxes for display.
- Add optional `checks.yaml` for machine-verifiable criteria.
- Make acceptance criteria fully structured and generate Markdown from them.

The reset should start with Markdown because task authors need low-friction prose. Add `checks.yaml` only when an automation consumer demands stronger typing.

### 1.4 Should task history be event-sourced?

The v2 artifact introduces append-only logs, but the envelope remains a snapshot. Full event sourcing would make events the source of truth and materialize the envelope. That improves audit and conflict handling but increases implementation cost and read complexity. The reset should stop at snapshot plus append-only audit unless sync conflict data proves it insufficient.

### 1.5 How much local execution context belongs in a task?

`workspace_path` and `repo_root` are currently persisted in task YAML. In a shared task world, those are often local bindings, not task identity. The open question is whether to store them in a local overlay keyed by task ID, or keep a portable workspace selector in the task envelope and resolve it locally at runtime.

### 1.6 How should old task IDs appear in commits?

After migration, old commits still contain `[T...]`. New commits should use `[ORB-A0001]`, but tasks with legacy history may benefit from dual tags during a transition:

```text
[ORB-A0001] [legacy task ID] Reset task schema
```

That is noisy. A better approach may be to teach `orbit task show` and search surfaces to resolve aliases and keep commit messages canonical going forward.

### 1.7 Are review threads task artifacts or PR artifacts?

Orbit currently stores review threads on tasks and can sync them with GitHub review comments. The v2 design moves them into task-local thread files. A future GitHub-native review mode may instead treat PR comments as the canonical source and task threads as a projection. The answer depends on whether Orbit remains the source of review truth or simply mirrors host review systems.

### 1.8 Should archived tasks be compacted?

Status-neutral directories make old tasks easy to keep forever. That is good for audit but bad for repository size. Possible compaction strategies:

- Keep the envelope and summaries, drop bulky artifacts.
- Move archived tasks into a cold store.
- Generate a signed archive bundle and remove the live task directory.

No compaction should land until retention and audit requirements are clearer.

---

## 2. Prior Work

### 2.1 Orbit task store

The existing `orbit-store::file::task_store` implementation is the baseline. It proves that directory-per-task bundles are inspectable, easy to back up, and easy for agents to reason about. Its main weakness is that it overuses `task.yaml` as both metadata envelope and append log.

### 2.2 Orbit ADR artifacts

[docs/design/adr-artifact/](../adr-artifact/) uses the same envelope-plus-Markdown pattern this design recommends. The ADR design also introduces globally unique monotonic IDs (`ADR-NNNN`) and legacy ID aliases. That is the closest internal precedent for `ORB-A0001` plus `legacy_ids`.

### 2.3 Orbit task sync

[docs/design/task-sync/](../task-sync/) documents a git-orphan-branch registry around the current task layout. It preserves `T<YYYYMMDD>-<N>` and status directories because it was designed as an incremental sync layer. The task reset intentionally reopens those decisions.

### 2.4 Issue trackers

Jira, Linear, and GitHub Issues separate short metadata from long discussion and activity streams. Their IDs are globally meaningful inside a tenant or repository. Their weakness for Orbit is that most relation graphs are hand-authored and sparse, which makes automation unreliable.

### 2.5 Git-backed trackers

Tools like `git-bug` show that issue/task data can live inside git without a server. Their strongest lesson for Orbit is that append-friendly records and stable IDs matter more than mimicking a SaaS issue tracker schema.

### 2.6 Event sourcing and CRDTs

Event sourcing treats the event log as the source of truth. CRDTs provide mathematically convergent replicated state. Orbit does not need the full machinery for every task field, but append-only comments and history should borrow the same instincts: append rather than rewrite, merge by stable event IDs, and avoid last-writer-wins for human prose.

---

## 3. What May Be Distinctive

### 3.1 Agent-readable intent as a first-class artifact

Most issue trackers optimize for human triage and reporting. Orbit tasks optimize for agents that need to resume work safely. The artifact should make intent, context, plan, validation, and residual risk available without requiring the agent to scrape chat history.

### 3.2 Local-first with an upgrade path to shared authority

The same task format should work in a single local workspace, an opt-in git registry, and a hosted team product. That is why ID allocation is explicit and why local execution bindings should not be baked into portable task identity.

### 3.3 Searchable prose with structured edges

The design treats prose as Markdown, not as unstructured junk. Search and semantic indexing read it field-by-field. Relations are structured separately so agents can traverse dependencies and lineage without text-parsing prose.

### 3.4 Audit without making YAML unreadable

The reset keeps `task.yaml` readable by moving audit traffic into append-only logs. That preserves the "open the file and understand the task" experience while giving audit and sync systems a better substrate.

---

## 4. References

### 4.1 Orbit-internal

- [docs/design/CONVENTIONS.md](../CONVENTIONS.md) - design folder layout and ADR rules.
- [docs/design/task-sync/](../task-sync/) - current sync proposal over the existing task layout.
- [docs/design/adr-artifact/](../adr-artifact/) - internal precedent for global IDs plus envelope and Markdown body.
- [docs/design/task-lineage/](../task-lineage/) - typed task relationships and derivation questions.
- [docs/design/semantic-search/](../semantic-search/) - per-field indexing of task text.
- [docs/POSITIONING.md](../../POSITIONING.md) - product doctrine that currently treats task IDs as local search keys.

### 4.2 External

- GitHub Issues - repository-scoped issue IDs, comments, timeline events, and PR linking.
- Linear - globally meaningful workspace issue identifiers, typed relations, and workflow states.
- Jira - project-key issue IDs and rich workflow metadata.
- `git-bug` - git-backed issue tracker using repository data as the persistence substrate.
- Martin Kleppmann, *Designing Data-Intensive Applications* - event logs, replication, and conflict-resolution background.

---

## Task References

None.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
