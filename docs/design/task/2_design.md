# Task — Design

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-05-11

This document describes the current task artifact implementation and the target v2 shape. The current implementation is still the source of truth until a cutover task lands. The v2 sections are intentionally prescriptive: they define the schema direction the next implementation should converge on instead of preserving every historical convenience.

---

## 1. Current Implementation

### 1.1 Storage layout

The current store is file-backed under `.orbit/tasks/`. Non-terminal statuses live directly under status directories:

```text
.orbit/tasks/
  proposed/<task-id>/
  backlog/<task-id>/
  someday/<task-id>/
  in_progress/<task-id>/
  review/<task-id>/
  blocked/<task-id>/
```

Terminal statuses are date-partitioned:

```text
.orbit/tasks/
  done/<yyyy-mm>/<task-id>/
  archived/<yyyy-mm>/<task-id>/
  rejected/<yyyy-mm>/<task-id>/
```

Each task directory contains:

- `task.yaml`
- `plan.md`
- `execution-summary.md`
- `artifacts/`

This is implemented in [crates/orbit-store/src/file/task_store/layout.rs](../../../crates/orbit-store/src/file/task_store/layout.rs) and [constants.rs](../../../crates/orbit-store/src/file/task_store/constants.rs).

### 1.2 Current YAML envelope

`task.yaml` currently carries both metadata and content. The persisted document includes:

- Identity: `id`, `parent_id`, `type`, `priority`, `complexity`.
- Content: `title`, `description`, `acceptance_criteria`, `dependencies`, `tags`.
- Context: `context_files`, `workspace_path`, `repo_root`.
- Ownership: `created_by`, `planned_by`, `implemented_by`.
- Implementation metadata: `model`, `pr_status`, `external_refs`.
- Attribution: `source_task_id`, `batch_id`.
- Timestamps: `created_at`, `updated_at`.
- Audit arrays: `history`, `comments`, `review_threads`.

The store splits only `plan` and `execution_summary` into Markdown companions. On read, [bundle.rs](../../../crates/orbit-store/src/file/task_store/bundle.rs) reconstructs a flat `Task` with string fields for `description`, `plan`, and `execution_summary`.

### 1.3 Current ID allocation

Current IDs have the form `T<YYYYMMDD>-<N>`, with optional legacy numeric suffixes. Allocation scans task directories for the current date and increments the highest suffix. The local task allocation lock serializes concurrent writers on one machine, but the allocator has no cross-workspace authority.

The validator rejects any other ID shape. That makes the format deeply embedded in lookup, lock, update, artifact, and delete entry points.

### 1.4 Current mutation model

Task mutations read a full bundle, update in memory, and rewrite the bundle. Lifecycle transitions also move the task directory between status paths. Comments, history entries, and review threads are YAML arrays, so append-only operations rewrite `task.yaml`.

This model is simple and inspectable on one machine. It becomes awkward under sync because directory moves and YAML-array appends create avoidable conflicts.

---

## 2. Target V2 Artifact Shape

### 2.1 Layout

The target v2 task bundle is status-neutral:

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
        RT-0001.yaml
        RT-0001.md
      artifacts/
        manifest.yaml
        files/
```

The partition directory is `floor(n / 100)` for the numeric suffix `n`, zero-padded to three digits for the initial five-digit range. Status lives in `task.yaml`. Directory moves are not part of lifecycle transitions. Read-side indexes or generated views can present status groupings for humans and dashboards.

This workspace path is the task working copy. It should remain ignored by Git by default, while ADRs and design docs remain committed project memory.

### 2.2 Envelope schema

`task.yaml` should be small and structured:

```yaml
schema_version: 5
id: ORB-00001
title: Reset task artifact schema
status: proposed
type: feature
priority: high
complexity: hard
relations:
  - type: supersedes
    target: ORB-00000
context_files:
  - file:crates/orbit-store/src/file/task_store/doc.rs
external_refs: []
created_by: codex:gpt-5.5
planned_by: null
implemented_by: null
created_at: 2026-05-11T00:00:00Z
updated_at: 2026-05-11T00:00:00Z
```

The envelope should not include prose bodies, comments, review message bodies, or execution summaries.

### 2.3 Prose documents

The v2 store treats these sidecars as first-class documents:

| File | Document name | Writer |
|------|---------------|--------|
| `description.md` | `description` | Task author or planning agent |
| `acceptance.md` | `acceptance` | Task author, planning agent, reviewer |
| `plan.md` | `plan` | Planner or executing agent |
| `execution-summary.md` | `execution-summary` | Implementing agent |

`acceptance.md` should prefer Markdown task list syntax:

```markdown
- [ ] Behavior X is implemented.
- [ ] Regression Y is covered.
- [ ] `make ci` passes.
```

APIs should treat the Markdown file as the source of truth. They may offer parsed checkbox views for UI convenience, but v2 storage should not preserve the old YAML array as a compatibility contract.

### 2.4 Append-only logs

`events.jsonl` records lifecycle and metadata events. Each row includes at least:

```json
{"event_id":"EV-0001","at":"2026-05-11T00:00:00Z","by":"codex:gpt-5.5","type":"created","from_status":null,"to_status":"proposed","note":null}
```

`comments.jsonl` records task comments:

```json
{"comment_id":"C-0001","at":"2026-05-11T00:00:00Z","by":"daniel","body":"Start over from the schema reset."}
```

Review threads get one metadata envelope and one Markdown body per thread. This keeps thread state structured while making review prose easy to read and diff.

### 2.5 Artifact manifest

Artifacts should support text and binary files. The current `TaskArtifact { path, content }` model only accepts UTF-8 text. V2 artifacts should use `artifacts/manifest.yaml`:

```yaml
schema_version: 1
files:
  - path: planning-duel/winner.json
    blob: files/planning-duel/winner.json
    media_type: application/json
    sha256: "<hex>"
    created_by: codex:gpt-5.5
```

Text artifacts may still be rendered inline by `orbit.task.show --field artifacts`, but storage should not require UTF-8.

### 2.6 Local backup layer

Local-first Orbit should keep a durable backup under `~/.orbit/tasks/` so the repo-local `.orbit/tasks/` tree can be rebuilt after accidental deletion, checkout recreation, or workspace cleanup. The backup store uses the same bundle payload shape as the workspace materialization:

```text
~/.orbit/tasks/
  index.sqlite
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

`index.sqlite` is the local allocation and recovery index. It should track canonical IDs, workspace bindings, materialized bundle paths, update timestamps, and enough checksums or revision markers to detect divergence between the home-directory backup and the workspace copy.

Write paths should update the backup layer and the workspace materialization as one logical operation. A task mutation should not report success while the only fresh copy exists under `.orbit/tasks/`. If Orbit finds that `.orbit/tasks/` is missing, it can restore all bundles bound to the workspace from `~/.orbit/tasks/`; if both copies exist but diverge, repair should merge append-only logs by stable IDs and require an explicit conflict decision before overwriting prose documents or envelopes.

---

## 3. ID Allocation

### 3.1 Format

The target canonical ID format is:

```text
ORB-00000
```

`ORB` names the product namespace. The suffix is decimal, zero-padded to five digits for the initial range (`ORB-00000` through `ORB-99999`). The string is the identity; consumers may parse the numeric suffix only for validation, ordering, and storage partitioning. If the corpus ever outgrows the five-digit range, a later ADR can define width expansion while preserving the numeric-only shape.

### 3.2 Storage partition

Task bundles live under a partition directory derived from the numeric suffix:

```text
.orbit/tasks/000/ORB-00000/  # ORB-00000 ... ORB-00099
.orbit/tasks/001/ORB-00100/  # ORB-00100 ... ORB-00199
.orbit/tasks/999/ORB-99999/  # ORB-99900 ... ORB-99999
```

The partition is `floor(n / 100)`, zero-padded to three digits for the initial five-digit range. Partitioning is an on-disk layout detail, not part of the public task identity.

### 3.3 Allocation authority

Global uniqueness requires an authority:

- Local-only OSS can allocate from the global local backup index under `~/.orbit/tasks/index.sqlite`, guarded by a process/file lock.
- Synced workspaces allocate against the task registry before materializing a bundle.
- Hosted team mode allocates through the hosted API.

The implementation should not claim global uniqueness by scanning one workspace's `.orbit/tasks/` tree. The allocator and backup store are related responsibilities but not the same logical API: allocation reserves an ID, while backup persistence makes an existing task recoverable.

### 3.4 No legacy aliases

V2 task bundles do not carry `legacy_ids`, and lookup surfaces should not resolve old `T<YYYYMMDD>-<N>` values. During a one-time cutover, Orbit may print a local report that maps old IDs to new IDs, but that report is an operator aid rather than part of the task schema. New commits and docs should cite only `ORB-00000` IDs.

---

## 4. API Contract

The public task API should expose the v2 bundle model directly:

- envelope metadata from `task.yaml`;
- Markdown documents by document name (`description`, `acceptance`, `plan`, `execution-summary`);
- append-only streams for events and comments;
- review-thread metadata plus Markdown message bodies;
- artifact manifest entries.

CLI and tool selectors may keep friendly names such as `description`, `plan`, and `execution-summary`, but they should be implemented as first-class document reads and writes. New internal code should operate on a bundle abstraction with explicit envelope, document, log, thread, and artifact fields.

---

## 5. Search and Indexing

Lexical and semantic search should index each logical field independently:

- `title`
- `description`
- `acceptance`
- `plan`
- `execution-summary`
- `comments`
- `review_threads`
- selected artifact text, when media type permits

This preserves field-aware semantic search while making file boundaries visible in snippets. The embedding index should store field names that match the v2 document names.

---

## 6. Cutover Path

The reset should be a one-time cutover rather than a long-lived compatibility layer:

1. Allocate or derive a canonical `ORB-00000` ID for every existing task.
2. Write or update the backup bundle under `~/.orbit/tasks/<partition>/<canonical-id>/`.
3. Materialize each task into `.orbit/tasks/<partition>/<canonical-id>/`.
4. Move `description` from `task.yaml` to `description.md`.
5. Render `acceptance_criteria` into `acceptance.md`.
6. Keep existing `plan.md` and `execution-summary.md`.
7. Move `history` into `events.jsonl`.
8. Move `comments` into `comments.jsonl`.
9. Move `review_threads` into `review-threads/`.
10. Rewrite `task.yaml` as the v2 envelope without old IDs or embedded prose.
11. Record the workspace binding in `~/.orbit/tasks/index.sqlite`.
12. Rebuild task tag, semantic, and any future relation indexes from disk.

The cutover command may emit a human-readable mapping from old IDs to new IDs, but Orbit should not persist that mapping as a lookup surface.

---

## 7. Concerns & Honest Limitations

Global IDs introduce a real allocator dependency. Local-only Orbit can keep that dependency offline and cheap, but sync and hosted modes need a shared authority. That is a deliberate tradeoff: a globally meaningful ID cannot be produced by a workspace-local scan.

Markdown acceptance criteria are friendlier to authors but weaker than typed checks. Automation gates that require structured checks should add a future `checks.yaml` instead of keeping the old YAML array alive.

Status-neutral directories simplify sync but make filesystem browsing less convenient. Humans lose the quick `ls .orbit/tasks/review` view unless Orbit generates indexes or CLI views. That cost is acceptable because lifecycle state belongs in the record, not in the path.

Append-only logs improve merge behavior but add more files per task. Small tasks become slightly noisier on disk. The benefit is that high-traffic tasks stop rewriting one large YAML file for every comment or event.

Binary artifacts increase storage flexibility and require stronger validation. Checksums, media type inference, size limits, and redaction rules become part of the artifact contract instead of being avoided by UTF-8-only writes.

The local backup layer prevents `.orbit/tasks/` deletion from being catastrophic, but it adds a second writable copy. Orbit needs explicit repair rules, revision checks, and diagnostics for cases where a user edits the workspace copy while the backup is stale or unavailable.

---

## Task References

- [T20260505-12] — Designed git-orphan-branch task sync and documented the current sync-era task bundle assumptions.
- [T20260506-11] — Removed knowledge-graph task attribution and documented why old task IDs were only local search keys.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
