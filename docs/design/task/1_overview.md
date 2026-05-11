# Task — Overview

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-05-11

Tasks are Orbit's durable intent records: they explain what an agent or human is trying to change, how the work should be validated, what context is relevant, who acted on the work, and how the work connects to other Orbit artifacts. The current implementation stores task bundles as YAML plus Markdown sidecars under `.orbit/tasks/`; this folder documents that implementation and the v2 reset direction that moves prose into Markdown files, narrows YAML to metadata, replaces date-scoped local IDs with an explicit global task ID format, and backs the repo-local working copy with durable task storage under `~/.orbit`.

This document is the entry point. [2_design.md](./2_design.md) describes the current store and the target v2 artifact shape; [3_vision.md](./3_vision.md) names open questions and prior work; [4_decisions.md](./4_decisions.md) captures the design decisions that should constrain implementation.

---

## 1. Motivation

Orbit tasks sit between transient agent sessions and durable code history. A good task artifact has to satisfy four different readers:

- Humans need a plain-English record of intent, acceptance criteria, and review context.
- Agents need a stable, machine-readable handoff surface that survives session loss.
- Automation needs structured fields for lifecycle, dependencies, locks, PR generation, semantic search, and audit.
- Git history needs a compact identifier that can be cited in commits and traced later.

The current schema got Orbit this far, but it mixes those concerns too tightly. `task.yaml` carries human prose (`description`, `acceptance_criteria`) next to metadata and append-heavy arrays (`history`, `comments`, `review_threads`). The directory path encodes lifecycle state, so status changes become file moves. The `T<YYYYMMDD>-<N>` ID format is easy to allocate locally but intentionally not globally resolvable; that assumption was made explicit after graph task attribution was removed in [T20260506-11].

Orbit is still pre-release, so the reset should not preserve historical compatibility as a product constraint. Existing local task records may be converted or discarded during a deliberate cutover, but v2 readers and writers should not carry alias lookup, dual schemas, or fallback projections indefinitely.

The reset opportunity is to make the artifact match the work:

- YAML should be a small, mergeable envelope.
- Markdown should hold prose and long-form human/agent reasoning.
- Append-only logs should carry events, comments, and review traffic.
- IDs should be stable across workspaces when tasks are intentionally shared.
- Workspace task bundles should be recoverable from a local backup layer, not treated as irreplaceable files inside the checkout.
- Local execution bindings should stay local, not leak into synced task identity.

---

## 2. Core Concepts

### 2.1 Task bundle

A task bundle is the complete on-disk representation of one task. In the current implementation, it is a directory containing `task.yaml`, `plan.md`, `execution-summary.md`, and optional `artifacts/`. In the v2 target shape, the bundle becomes:

```text
.orbit/tasks/
  000/
    ORB-00000/
      task.yaml
      description.md
      acceptance.md
      plan.md
      execution-summary.md
      events.jsonl
      comments.jsonl
      review-threads/
      artifacts/
```

The final directory name is the task ID. The parent directory is a deterministic partition derived from the numeric suffix. Status is stored in `task.yaml`; it is not encoded in the path in the v2 target shape.

### 2.2 Envelope

`task.yaml` is the envelope. It owns identity, lifecycle state, priority, type, typed relations, context selectors, external references, actor attribution, timestamps, and schema version. It should not carry long prose or append-heavy conversation history in the v2 artifact.

### 2.3 Prose sidecars

Long-form fields are Markdown sidecars:

- `description.md` explains the problem, desired behavior, and relevant context.
- `acceptance.md` records validation expectations, preferably as Markdown checkboxes.
- `plan.md` records the execution plan.
- `execution-summary.md` records what changed, validation performed, and residual risk.

The public API should expose these as first-class task documents, using names that match the v2 sidecars instead of preserving old embedded-YAML field shapes.

### 2.4 Global task ID

The proposed v2 ID format is `ORB-00000`: `ORB` identifies Orbit and the five-digit decimal suffix is globally unique within the configured allocation authority. Storage partitions by hundreds: `ORB-00000` through `ORB-00099` live under `.orbit/tasks/000/`, `ORB-00100` through `ORB-00199` live under `.orbit/tasks/001/`, and so on. A local-only workspace may use a local global allocator in `~/.orbit`; a synced or hosted workspace must allocate against the shared registry before the task is visible.

Old `T<YYYYMMDD>-<N>` values are not part of the v2 identity contract. A one-time cutover may map current local tasks to new IDs for operator convenience, but Orbit should not expose old IDs as supported aliases.

### 2.5 Task event stream

Lifecycle changes, comments, review messages, and automation updates are naturally append-heavy. The v2 artifact moves those out of YAML arrays into append-only JSON Lines files or per-thread records so concurrent writes can be merged intentionally and audited without rewriting the whole envelope.

### 2.6 Typed relations

The current model has several separate edge-like fields (`parent_id`, `dependencies`, `source_task_id`, `batch_id`). The v2 target collapses these into typed relations such as `parent_of`, `blocks`, `blocked_by`, `spawned_from`, `regression_from`, `supersedes`, and `related_to`. Consumers should reason about relation meaning rather than infer it from field names.

### 2.7 Local backup store

The v2 workspace bundle is a materialized working copy, not the only durable copy of the task. Local-first Orbit should keep a backup store under `~/.orbit/tasks/` with the same task bundle payloads plus allocator, checksum, and workspace-binding indexes. If `.orbit/tasks/` is deleted from a checkout, Orbit should be able to restore the materialized bundles from `~/.orbit` for that workspace.

---

## 3. At a Glance

| Concern | File | Task |
|---------|------|------|
| Current task schema and public `Task` struct | [crates/orbit-common/src/types/task.rs](../../../crates/orbit-common/src/types/task.rs) | — |
| Current file-backed task store | [crates/orbit-store/src/file/task_store/](../../../crates/orbit-store/src/file/task_store/) | — |
| Current bundle constants | [crates/orbit-store/src/file/task_store/constants.rs](../../../crates/orbit-store/src/file/task_store/constants.rs) | — |
| Current YAML document fields | [crates/orbit-store/src/file/task_store/doc.rs](../../../crates/orbit-store/src/file/task_store/doc.rs) | — |
| Current local ID allocation and validation | [crates/orbit-store/src/file/task_store/layout.rs](../../../crates/orbit-store/src/file/task_store/layout.rs) | — |
| Target local backup layer | [2_design.md §2.6](./2_design.md#26-local-backup-layer) | — |
| Task sync design that preserves current `T<YYYYMMDD>-<N>` IDs | [docs/design/task-sync/2_design.md](../task-sync/2_design.md) | [T20260505-12] |
| Task ID local-only doctrine after graph attribution removal | [docs/POSITIONING.md](../../POSITIONING.md), [knowledge-graph/4_decisions.md](../knowledge-graph/4_decisions.md) | [T20260506-11] |
| v2 task bundle contract | [specs/task-bundle-v2.md](./specs/task-bundle-v2.md) | — |
| Glossary | [references/glossary.md](./references/glossary.md) | — |
| ADR log | [4_decisions.md](./4_decisions.md) | — |

---

## Task References

- [T20260505-12] — Designed git-orphan-branch task sync and preserved `T<YYYYMMDD>-<N>` for that proposal.
- [T20260506-11] — Removed knowledge-graph task attribution and made task IDs local search keys rather than cross-machine references.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
