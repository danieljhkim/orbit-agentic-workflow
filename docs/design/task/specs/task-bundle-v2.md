# Spec: Task Bundle V2

Task Bundle V2 defines the canonical on-disk contract for Orbit tasks after the schema reset. The contract is source-of-truth storage, not merely a rendering preference: implementations may expose compatibility fields, but persisted data follows the bundle shape below.

## Why This Exists

The current task bundle is easy to inspect but hard to sync and evolve. Long prose, append-heavy audit arrays, status directory moves, and workspace-local IDs all interact badly with shared registries and long-lived agent workflows. This spec pins the reset target so migrations and new code converge on one artifact shape.

## Bundle Path

Every task bundle lives at:

```text
.orbit/tasks/<task-id>/
```

`<task-id>` must be the canonical ID, not a legacy alias. Legacy IDs resolve through the alias index or `legacy_ids` field.

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

## Envelope

`task.yaml` must contain only structured metadata:

```yaml
schema_version: 5
id: ORB-A0001
legacy_ids: []
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

Compatibility projections:

- `description` returns `description.md` as a string.
- `acceptance_criteria` parses non-empty checkbox or bullet lines from `acceptance.md`.
- `plan` returns `plan.md` as a string.
- `execution_summary` returns `execution-summary.md` as a string.

## Events

`events.jsonl` is append-only. Each line is a JSON object with:

- `event_id`: stable per-task event ID.
- `at`: RFC 3339 timestamp.
- `by`: actor label.
- `type`: event type.
- `note`: optional string.
- `from_status`: optional status.
- `to_status`: optional status.

Writers must not rewrite prior event lines except during explicit migration or repair commands.

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

## Migration

Migration from schema version 4 must:

1. Allocate a canonical `ORB-A0001` ID.
2. Preserve old `id` in `legacy_ids`.
3. Move YAML `description` to `description.md`.
4. Render YAML `acceptance_criteria` into `acceptance.md`.
5. Preserve existing `plan.md`.
6. Preserve existing `execution-summary.md`.
7. Convert YAML `history` to `events.jsonl`.
8. Convert YAML `comments` to `comments.jsonl`.
9. Convert YAML `review_threads` to `review-threads/`.
10. Rewrite `task.yaml` with schema version 5.

Migration must be idempotent. A partially migrated task must either repair cleanly on rerun or fail with a diagnostic that names the task ID and incomplete step.

## Agent Signature

Last revised by `codex` on 2026-05-11.
