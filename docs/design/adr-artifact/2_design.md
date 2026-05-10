# ADR Artifact — Design

**Status:** Draft
**Owner:** claude
**Last updated:** 2026-05-10

This document specifies the v2 implementation: artifact shape, storage layout, tool contracts, lifecycle transitions, migration mechanics, and the boundaries that keep `orbit-store` and `orbit-tools` cleanly scoped. v1 ships none of this; the design is captured now so the migration and tooling are not invented under deadline pressure. See [1_overview.md](./1_overview.md) for purpose and [3_vision.md](./3_vision.md) for open questions.

---

## 1. Artifact Shape

An ADR is a per-ADR directory containing a YAML envelope and a markdown body. The envelope holds structured metadata; the body holds the human-readable Context / Decision / Consequences prose. Splitting them keeps metadata queryable without parsing markdown, and keeps writing experience close to today's ADR template.

```
.orbit/adrs/accepted/ADR-0042/
├── adr.yaml      # envelope (structured metadata)
└── body.md       # Context / Decision / Consequences prose
```

`adr.yaml`:

```yaml
id: ADR-0042
title: Resolve sandbox-exec wrapper from a trusted absolute path
status: accepted
owner: codex
created_at: 2026-05-09T14:22:00Z
accepted_at: 2026-05-09T18:01:00Z
last_updated: 2026-05-09T18:01:00Z
related_features: [activity-job, policy-sandbox]
related_tasks: [T20260509-30]
supersedes: []
superseded_by: null
legacy_id: activity-job/ADR-039
```

`body.md`:

```markdown
## Context
<1–3 sentences. Why this forced a decision.>

## Decision
<1–3 sentences. What we chose.>

## Consequences
- <bullet>
- Cost: <explicit tradeoff>
```

The directory-per-ADR layout matches `orbit-store::task_store`, which uses `<status>/<yyyy-mm>/<id>/task.yaml` plus companion markdown files (`plan.md`, `execution-summary.md`) and an optional `artifacts/` subtree. Reusing the pattern means no new store primitives and lets future attachments (diagrams, supplementary specs, review threads) live next to the ADR without changing the storage contract. See [ADR-005](./4_decisions.md#adr-005--directory-per-adr-with-yaml-envelope-and-markdown-body).

---

## 2. ID Allocation

IDs are globally unique and monotonically allocated: `ADR-NNNN`, zero-padded to four digits initially (`ADR-0001`). When NNNN exceeds 9999 the pad grows; references already written remain valid because the string is the ID, not the integer.

Allocation is single-writer per workspace: `orbit.adr.add` opens the index, reads the max existing ID, increments, and writes. Same approach as task ID allocation in `orbit-store::file::task_store::layout` — workspace-local, no coordination needed.

**No per-feature numbering.** A decision that touches three features has one ID, referenced from three `2_design.md` files. Migration assigns new global IDs and records the legacy per-feature ID in `legacy_id` so historical references (`docs/design/activity-job/4_decisions.md` cited in commits) still resolve via `orbit.adr.list --legacy-id=...`.

---

## 3. Storage and Scoping

Files live under `.orbit/adrs/<status>/<id>/{adr.yaml,body.md}`. Status-directory layout mirrors `task_store`:

```
.orbit/adrs/
├── proposed/
│   └── ADR-0098/
│       ├── adr.yaml
│       └── body.md
├── accepted/
│   └── ADR-0042/
│       ├── adr.yaml
│       └── body.md
└── superseded/
    └── ...
```

A SQLite index at `.orbit/adrs/index.sqlite` mirrors envelope fields plus an FTS5 column over the body, enabling fast list/filter without scanning every directory. The index is rebuildable from the YAML+markdown files — those are the source of truth.

**Scoping: `WorkspaceOnly`** initially, matching Tasks. Cross-workspace ADRs (org-wide architectural patterns) are deferred — see [3_vision.md §1](./3_vision.md). The decision is additive: switching to `MergeByKey` later doesn't break existing workspace-local ADRs.

---

## 4. Tool Surface

Six tools, contracts below. All return structured JSON; CLI surfaces are thin wrappers.

### 4.1 `orbit.adr.add`

Creates a `proposed` ADR. Input: `title`, `owner`, `related_features`, optional `related_tasks`, optional initial body sections. Output: assigned `id`. Errors: invalid feature name, missing required field.

### 4.2 `orbit.adr.show`

Input: `id`. Output: full envelope + body. Errors: not found.

### 4.3 `orbit.adr.list`

Input: optional filters (`feature`, `status`, `owner`, `task_id`, `since`). Output: array of envelopes (no body). Sort: by `id` descending by default.

### 4.4 `orbit.adr.update`

Input: `id`, plus any subset of (`status`, body sections, `related_tasks`, `related_features`, `owner`). Status transitions enforced:

- `proposed → accepted` requires non-empty `related_tasks` on the update payload (the task that shipped it).
- `proposed → superseded` and `accepted → superseded` go through `adr.supersede` instead — direct status writes to `superseded` are rejected.
- `accepted → proposed` is rejected. A reverted ADR is superseded by a new ADR explaining the reversal.

### 4.5 `orbit.adr.supersede`

Input: `old_id`, `new_id`. Effects: sets `old.status = superseded`, `old.superseded_by = new_id`, appends `old_id` to `new.supersedes`. Both records' `last_updated` bump. Errors: either ID missing, `new_id` not `accepted`.

### 4.6 `orbit.adr.search`

Input: `query` string, optional filters. Output: ranked array of `{id, title, score, snippet}`. Implementation: reuses `orbit-embed::vector::VectorStore` with a new `adrs` collection alongside the existing `tasks_fts` schema. No new embedding infrastructure.

### 4.7 `orbit.adr.review_thread.{add, list, reply, resolve}`

Mirrors `orbit.task.review_thread.*`. Threads are scoped to a single ADR by `adr_id`; storage attaches under the per-ADR directory at `.orbit/adrs/<status>/<id>/review_threads/<thread_id>.yaml`, preserving the directory-per-ADR pattern. Whether the `proposed → accepted` transition blocks on unresolved threads is deferred — see [ADR-009](./4_decisions.md#adr-009--review-threads-on-adrs).

---

## 5. Lifecycle and Audit

State machine is small:

```
        proposed ──(adr.update --status=accepted)──> accepted
            │                                            │
            │                                            │
            └──(adr.supersede)──> superseded <──(adr.supersede)──┘
```

Every transition writes an audit row to the existing command-audit SQLite database (Scoping: `GlobalOnly` per CLAUDE.md). Rows carry `adr_id`, `from_status`, `to_status`, `actor` (agent identity), `task_id` (when applicable), `timestamp`. This reuses the audit surface activities/jobs already write to — no new event taxonomy.

Deletion of `proposed` ADRs is permitted via `orbit.adr.update --status=deleted` (sets `status=deleted`, leaves the file in place under `.orbit/adrs/deleted/` for archaeology). Accepted ADRs cannot be deleted, only superseded.

---

## 6. Reference Syntax and Enforcement

Design docs cite ADRs inline as plain bracketed text: `[ADR-0042]`. This matches the existing `[T20260421-0528]` convention for task IDs (never linked, always plain — per [CONVENTIONS.md §8](../CONVENTIONS.md)) and avoids brittle relative-path links.

Enforcement is opt-in via a lint that the migration task ships alongside the store:

- Every `[ADR-NNNN]` cited in `docs/design/**/*.md` must resolve through `orbit.adr.show`.
- Every `[ADR-NNNN]` cited with status `superseded` produces a warning, not an error — superseded references are legitimate when the doc is itself historical.

CONVENTIONS.md is updated in the migration task to reflect this. The lint runs in `make ci`.

---

## 7. Migration

A one-shot tool walks every `docs/design/*/4_decisions.md`, parses each ADR entry against the existing markdown template, and writes one artifact per entry:

1. **Parse.** Each `## ADR-NNN — <title>` heading starts a record. Status line, Context, Decision, Consequences are extracted by section heading. Rollup ADRs ([CONVENTIONS.md §4a](../CONVENTIONS.md)) collapse into one artifact with the rollup's body and the folded entries' costs preserved.
2. **Allocate.** Each parsed entry gets a fresh global ID; the source folder + per-feature number is written to `legacy_id`.
3. **Cross-reference.** `Supersedes` / `Superseded by ADR-MMM` lines in the source markdown become bidirectional `supersedes` / `superseded_by` edges on the new artifacts.
4. **Sweep `2_design.md`.** Inline `[T...]` change-history that maps cleanly to an ADR ("After [T20260427-34], `invoke_and_wait`...") is rewritten as `[ADR-NNNN]` citations. Lines without a corresponding ADR are left as-is — automated rewrite, not heuristic deletion.
5. **Convert `4_decisions.md` to a generated artifact.** Per [ADR-006](./4_decisions.md#adr-006--auto-generate-per-feature-4_decisionsmd-index), each feature's `4_decisions.md` is regenerated from the store via `orbit.adr.list --feature=<name> --format=md`. The migration tool emits the first generated version; subsequent updates run automatically. Cross-cutting ADRs ([ADR-007](./4_decisions.md#adr-007--cross-cutting-adrs-use-a-dedicated-cross-cutting-index)) populate `docs/design/cross-cutting/4_decisions.md` from the same generator.

The migration is idempotent: running it on an already-migrated workspace is a no-op (existing `legacy_id` matches → skip). This matters because the migration might land before every feature has been swept and re-runs may be needed.

---

## 8. Concerns & Honest Limitations

### 8.1 Markdown body still grows unboundedly

Lifting metadata to structured fields doesn't shrink the prose. An accepted ADR's Context / Decision / Consequences still bloats over time if owners aren't disciplined. The artifact makes growth queryable, not absent.

### 8.2 Bootstrap circularity in this folder

`docs/design/adr-artifact/4_decisions.md` is itself a markdown ADR log — the very artifact this proposal seeks to replace. It can only be migrated *into* the store once the store exists. v2's first ADR migration step is "migrate this folder."

### 8.3 ID renumbering breaks pre-migration references

Existing markdown commits cite `[ADR-017]` against a feature folder; after migration the same decision lives at `ADR-0042`. The `legacy_id` field makes the old reference resolvable, but only through `orbit.adr.list --legacy-id`, not via a plain grep over the new corpus. Tooling helps; muscle memory has to retrain.

### 8.4 Semantic search index lifecycle

`orbit.adr.search` requires the embedding companion to be installed. On workspaces without it, search returns `NoopEmbedder` results (empty or fallback FTS). This matches existing `orbit-embed` behavior — no new install requirement — but the degraded mode needs to be documented at the tool surface, not just here.

### 8.5 Cross-workspace decisions remain unsolved

`WorkspaceOnly` scoping means an architectural pattern that should apply to every Orbit consumer (e.g. *"prefer `OrbitError` over ad-hoc `String` errors"*) has no canonical home. CLAUDE.md fills that role today. Whether to lift those into a `MergeByKey` ADR space is an open question, not a v2 commitment.

### 8.6 Migration cannot fully automate the `2_design.md` sweep

Step 7.4 rewrites inline `[T...]` to `[ADR-NNNN]` where the mapping is unambiguous. Some change-history prose doesn't map to a single ADR — it's narrative spanning several decisions — and those passages need a human edit. Migration produces a report of remaining `[T...]` citations per feature so leads can finish the sweep manually.

---

## Task References

- [T20260510-27] — Drafted the adr-artifact design folder as a v2 proposal. This document specifies the artifact shape, ID allocation, storage layout, tool surface, lifecycle transitions, reference syntax, and migration mechanics.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
