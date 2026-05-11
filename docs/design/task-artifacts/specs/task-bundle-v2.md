# Spec: Task Bundle V2

Task Bundle V2 defines the canonical on-disk contract for Orbit tasks after the schema reset. The contract is source-of-truth storage, not merely a rendering preference: implementations should expose the bundle shape below rather than maintaining long-lived compatibility with the previous task schema.

## Why This Exists

The current task bundle is easy to inspect but hard to sync and evolve. Long prose, append-heavy audit arrays, status directory moves, and workspace-local IDs all interact badly with shared registries and long-lived agent workflows. This spec pins the reset target so cutover work and new code converge on one artifact shape.

## Bundle Path

Every task bundle lives at:

```text
.orbit/tasks/<partition>/<task-id>/
```

`<task-id>` must be the canonical ID. The canonical v2 format is `ORB-` plus a five-digit decimal suffix (`ORB-00000` through `ORB-99999`). `<partition>` is `floor(n / 100)` for the numeric suffix `n`, zero-padded to three digits. Old `T<YYYYMMDD>-<N>` IDs are not valid v2 identifiers or lookup aliases.

Required files:

```text
task.yaml
description.md
acceptance.md
plan.md
execution-summary.md
events.jsonl
comments.jsonl
```

Required directories:

```text
review-threads/
artifacts/
```

`artifacts/manifest.yaml` is required when `artifacts/files/` contains any files.

## Local Backup Store

The repo-local bundle is a materialized working copy. Local-first Orbit must keep a recoverable backup under:

```text
~/.orbit/tasks/<partition>/<task-id>/
```

The backup bundle uses the same required files and directories as the workspace bundle. The local backup root also owns an allocation and recovery index:

```text
~/.orbit/tasks/index.sqlite
```

The index must record canonical IDs, workspace bindings, materialized paths, update timestamps, and revision/checksum data sufficient to detect whether the workspace copy and backup copy have diverged.

Task mutations must update the backup layer before reporting success. If the workspace `.orbit/tasks/` tree is deleted, Orbit should restore bound task bundles from `~/.orbit/tasks/`. If both copies exist and disagree, repair may merge append-only logs by stable IDs, but it must not silently overwrite envelopes or Markdown documents.

## Envelope

`task.yaml` must contain only structured metadata:

```yaml
schema_version: 5
id: ORB-00000
title: Short title
status: proposed
type: chore
priority: medium
complexity: null
relations: []
tags: []
context_files: []
external_refs: []
created_by: codex:gpt-5.5
planned_by: null
implemented_by: null
created_at: 2026-05-11T00:00:00Z
updated_at: 2026-05-11T00:00:00Z
```

The envelope must not contain `description`, `acceptance_criteria`, `plan`, `execution_summary`, `history`, `comments`, or `review_threads`.

## Documents

Document sidecars are UTF-8 Markdown:

- `description.md` may be empty only for machine-generated placeholder tasks.
- `acceptance.md` may be empty while a task is `proposed`, but a task entering `in-progress` should have at least one bullet or checkbox unless the task type explicitly waives criteria.
- `plan.md` must be non-empty before entering `in-progress` when the transition policy requires a plan.
- `execution-summary.md` must be non-empty before entering `review`.

Convenience reads:

- `description` returns `description.md` as a string.
- `acceptance` returns `acceptance.md` as a string; UI code may parse checkboxes for display.
- `plan` returns `plan.md` as a string.
- `execution-summary` returns `execution-summary.md` as a string.

## Events

`events.jsonl` is append-only. Each line is a JSON object with:

- `event_id`: stable per-task event ID.
- `at`: RFC 3339 timestamp.
- `by`: actor label.
- `type`: event type.
- `note`: optional string.
- `from_status`: optional status.
- `to_status`: optional status.

Writers must not rewrite prior event lines except during explicit cutover or repair commands.

## Comments

`comments.jsonl` is append-only. Each line is a JSON object with:

- `comment_id`
- `at`
- `by`
- `body`

Comment bodies are Markdown strings. Multi-line bodies must be JSON escaped inside one JSONL row, not spread across lines.

## Review Threads

Each review thread uses:

```text
review-threads/<thread-id>.yaml
review-threads/<thread-id>.md
```

The YAML file stores status, file path, line/range metadata, external review IDs, timestamps, and message metadata. The Markdown file stores message bodies in chronological order with stable message anchors.

## Artifacts

Artifacts are stored under `artifacts/files/` and listed in `artifacts/manifest.yaml`:

```yaml
schema_version: 1
files:
  - path: planning-duel/winner.json
    blob: files/planning-duel/winner.json
    media_type: application/json
    sha256: "<hex>"
    size_bytes: 1234
    created_by: codex:gpt-5.5
    created_at: 2026-05-11T00:00:00Z
```

Artifact paths must be relative, UTF-8, slash-separated, and must not contain `..` components.

## Cutover

Cutover from schema version 4 must:

1. Allocate a canonical `ORB-00000` ID.
2. Create or update the backup bundle under `~/.orbit/tasks/<partition>/<task-id>/`.
3. Materialize the workspace bundle under `.orbit/tasks/<partition>/<task-id>/`.
4. Move YAML `description` to `description.md`.
5. Render YAML `acceptance_criteria` into `acceptance.md`.
6. Preserve existing `plan.md`.
7. Preserve existing `execution-summary.md`.
8. Convert YAML `history` to `events.jsonl`.
9. Convert YAML `comments` to `comments.jsonl`.
10. Convert YAML `review_threads` to `review-threads/`.
11. Rewrite `task.yaml` with schema version 5 and no old ID aliases.
12. Record workspace bindings in the local backup index.

Cutover must be idempotent for interrupted local runs. A partially converted task must either repair cleanly on rerun or fail with a diagnostic that names the task ID and incomplete step. The command may emit an old-ID-to-new-ID report for humans, but that report is not a persisted lookup contract.

## Agent Signature

Last revised by `codex` on 2026-05-11.
