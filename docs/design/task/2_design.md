# Task — Design

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-05-11

This document describes the current task artifact implementation and the target v2 shape. The current implementation is still the source of truth until a migration task lands. The v2 sections are intentionally prescriptive: they define the schema direction the next implementation should converge on instead of preserving every historical convenience.

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
  ORB-A0001/
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

Status lives in `task.yaml`. Directory moves are not part of lifecycle transitions. Read-side indexes or generated views can present status groupings for humans and dashboards.

### 2.2 Envelope schema

`task.yaml` should be small and structured:

```yaml
schema_version: 5
id: ORB-A0001
legacy_ids:
  - T20260510-17
title: Reset task artifact schema
status: proposed
type: feature
priority: high
complexity: hard
relations:
  - type: supersedes
    target: ORB-A0000
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

| File | Logical field | Writer |
|------|---------------|--------|
| `description.md` | `description` | Task author or planning agent |
| `acceptance.md` | `acceptance_criteria` | Task author, planning agent, reviewer |
| `plan.md` | `plan` | Planner or executing agent |
| `execution-summary.md` | `execution_summary` | Implementing agent |

`acceptance.md` should prefer Markdown task list syntax:

```markdown
- [ ] Behavior X is implemented.
- [ ] Regression Y is covered.
- [ ] `make ci` passes.
```

APIs may still expose acceptance criteria as an array for compatibility by parsing checkbox or bullet lines. The Markdown file is the source of truth.

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

---

## 3. ID Allocation

### 3.1 Format

The target canonical ID format is:

```text
ORB-A0001
```

`ORB` names the product namespace. `A` is an allocation series. The numeric suffix is zero-padded to four digits initially and grows when needed. The string is the identity; no consumer should parse business meaning from the number beyond validation and ordering.

### 3.2 Allocation authority

Global uniqueness requires an authority:

- Local-only OSS can allocate from a global local index under `~/.orbit`, guarded by a process/file lock.
- Synced workspaces allocate against the task registry before materializing a bundle.
- Hosted team mode allocates through the hosted API.

The implementation should not claim global uniqueness by scanning one workspace's `.orbit/tasks/` tree.

### 3.3 Legacy aliases

Migrated tasks carry `legacy_ids`. All lookup surfaces should accept a legacy ID and resolve it to the canonical ID when unambiguous. New commits should cite the canonical ID once a task has migrated, but historical `git log --grep '[T...]'` remains valid.

---

## 4. API Compatibility

The public `Task` struct can keep logical fields during migration:

- `description: String`
- `acceptance_criteria: Vec<String>`
- `plan: String`
- `execution_summary: String`

The store should treat those as projections over files, not proof that the files are embedded in YAML. CLI and tool surfaces should keep existing field selectors (`description`, `acceptance_criteria`, `plan`, `execution_summary`) while adding file-aware update paths where useful.

The compatibility layer should be intentionally thin. New internal code should operate on a bundle abstraction with explicit envelope and document fields.

---

## 5. Search and Indexing

Lexical and semantic search should index each logical field independently:

- `title`
- `description`
- `acceptance`
- `plan`
- `execution_summary`
- `comments`
- `review_threads`
- selected artifact text, when media type permits

This preserves the current per-field semantic indexing behavior while making file boundaries visible in snippets. The embedding index should store field names that match the document names where possible (`description`, not `summary`, unless a compatibility alias is required).

---

## 6. Migration Path

The migration should be mechanical:

1. Allocate or derive a canonical `ORB-A0001` ID for every existing task.
2. Move each task into `.orbit/tasks/<canonical-id>/`.
3. Preserve old ID in `legacy_ids`.
4. Move `description` from `task.yaml` to `description.md`.
5. Render `acceptance_criteria` into `acceptance.md`.
6. Keep existing `plan.md` and `execution-summary.md`.
7. Move `history` into `events.jsonl`.
8. Move `comments` into `comments.jsonl`.
9. Move `review_threads` into `review-threads/`.
10. Rewrite `task.yaml` as the v2 envelope.
11. Rebuild task tag, semantic, and any future relation indexes from disk.

The migration should leave an alias index so `orbit task show T20260510-17` remains useful after the task directory has moved.

---

## 7. Concerns & Honest Limitations

Global IDs introduce a real allocator dependency. Local-only Orbit can keep that dependency offline and cheap, but sync and hosted modes need a shared authority. That is a deliberate tradeoff: a globally meaningful ID cannot be produced by a workspace-local scan.

Markdown acceptance criteria are friendlier to authors but weaker than a typed array. The parser has to reject ambiguous shapes or expose a lossy projection. For automation gates that require structured checks, a future `checks.yaml` may be cleaner than overloading Markdown.

Status-neutral directories simplify sync but make filesystem browsing less convenient. Humans lose the quick `ls .orbit/tasks/review` view unless Orbit generates indexes or CLI views. That cost is acceptable because lifecycle state belongs in the record, not in the path.

Append-only logs improve merge behavior but add more files per task. Small tasks become slightly noisier on disk. The benefit is that high-traffic tasks stop rewriting one large YAML file for every comment or event.

Binary artifacts increase storage flexibility and require stronger validation. Checksums, media type inference, size limits, and redaction rules become part of the artifact contract instead of being avoided by UTF-8-only writes.

---

## Task References

- [T20260505-12] — Designed git-orphan-branch task sync and documented the current sync-era task bundle assumptions.
- [T20260506-11] — Removed knowledge-graph task attribution and made legacy task IDs local search keys.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
