# Semantic Search — Design

**Status:** Draft
**Owner:** claude
**Last updated:** 2026-05-09

This document specifies phase-1 semantic search: the new `orbit-embed` crate and its place in the architecture, the inference backend, the SQLite vector storage schema, the per-field embedding strategy, the hybrid (BM25 + cosine) retrieval pipeline, the MCP and CLI surface, the index-maintenance lifecycle, and the concerns the design deliberately leaves to follow-ups.

---

## 1. Architectural Placement

A new crate `orbit-embed` is added as a near-leaf in the dependency graph:

```
orbit-common → orbit-embed → orbit-store → ... (existing graph)
```

`orbit-embed` depends only on `orbit-common`. It owns the `Embedder` trait, the fastembed-rs-backed default implementation, model resolution, and the model-cache path policy. It does *not* depend on `orbit-store` — the store consumes the trait. This keeps inference machinery isolated from persistence and mirrors the way `orbit-policy` and `orbit-exec` are kept narrow.

`orbit-store` gains a new submodule `vector::` alongside the existing `file::` and `sqlite::` layers. `vector::` owns the `embeddings` table schema, write/upsert/delete API, and the brute-force cosine query implementation. It depends on `orbit-embed` for the `Embedder` trait but treats the embedder as injected — tests can pass a `NoopEmbedder` that returns zero vectors.

`orbit-tools` exposes `orbit.semantic.search` and `orbit.semantic.related` as MCP tools, and `orbit-cli` exposes `orbit semantic` subcommands. Both are thin shells over `orbit-store::vector::*`.

---

## 2. Inference Backend

### 2.1 Trait

```rust
pub trait Embedder: Send + Sync {
    fn model_id(&self) -> &str;        // e.g. "bge-small-en-v1.5"
    fn dim(&self) -> usize;            // e.g. 384
    fn max_input_tokens(&self) -> usize; // e.g. 512
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, OrbitError>;
}
```

Batch input is mandatory — fastembed-rs is meaningfully faster on batches than on single-document calls because of ONNX kernel reuse, and the indexing path naturally batches by task. Single-document `embed_one` is provided as a thin wrapper for query-time use.

### 2.2 Default implementation: `FastembedBackend`

Wraps fastembed-rs. Default model is **BGE-small-en-v1.5** (384 dim, ~30MB on disk, 512-token context). The choice is justified in [4_decisions.md ADR-001](./4_decisions.md); briefly, BGE-small sits at the favorable end of the recall/size/latency curve for English-language task content and is the default in the fastembed-rs release.

Model resolution order:

1. `~/.orbit/embed/models/<model_id>/` if pre-populated (airgapped).
2. fastembed-rs default cache (`$HF_HOME` or platform default).
3. Download from HuggingFace on first use, written to (1).

On a missing model in airgapped mode (no network + no preloaded cache), `embed()` returns a typed `OrbitError::EmbedderUnavailable` with a remediation message. Callers in the indexing path log and skip; query path surfaces a clear CLI/MCP error.

### 2.3 Alternative backends

The trait makes Candle and llama.cpp viable swap-ins without storage-layer changes. None ship in phase 1; the trait exists to keep that door open. The full backend comparison is in [4_decisions.md ADR-001](./4_decisions.md).

---

## 3. Vector Storage

### 3.1 Schema

A new SQLite table in the existing per-workspace store:

```sql
CREATE TABLE embeddings (
    source_kind TEXT NOT NULL,         -- "task" (phase 1); "symbol" (phase 2 reserved)
    source_id   TEXT NOT NULL,         -- task ID or future symbol ID
    field       TEXT NOT NULL,         -- "purpose", "plan", "comment_3", "review_1_msg_2", ...
    chunk_idx   INTEGER NOT NULL,      -- 0 for unchunked; >0 for splits of long fields
    content_hash TEXT NOT NULL,        -- BLAKE3 of the embedded text; cheap re-index gate
    model_id    TEXT NOT NULL,         -- "bge-small-en-v1.5"
    dim         INTEGER NOT NULL,      -- 384
    embedding   BLOB NOT NULL,         -- dim * 4 bytes, native-endian f32
    created_at  TEXT NOT NULL,
    PRIMARY KEY (source_kind, source_id, field, chunk_idx, model_id)
);

CREATE INDEX embeddings_by_source ON embeddings(source_kind, source_id);
CREATE INDEX embeddings_by_model  ON embeddings(model_id);
```

The composite primary key includes `model_id` so embeddings under multiple models can coexist during a model migration. The `content_hash` lets the indexer skip work when the underlying text hasn't changed since the last embed.

### 3.2 Query path (brute force)

```text
1. embed query under default model_id  → query vector q (dim 384)
2. SELECT embedding, source_kind, source_id, field, chunk_idx
     FROM embeddings WHERE model_id = ?
3. for each row: compute cosine(q, row.embedding); maintain a fixed-size top-k heap
4. return top-k (source_id, field, score)
```

At 30k vectors × 384d, the full scan is ~50ms in pure Rust on a modern laptop and dominated by the SQLite read, not the dot product. The implementation uses `f32` slabs and SIMD-friendly contiguous buffers; it does not need an HNSW index at phase-1 scale. The forward path to HNSW (via `sqlite-vec`) is preserved by the schema — `embedding BLOB` is the same shape `sqlite-vec` expects ([4_decisions.md ADR-002](./4_decisions.md)).

### 3.3 Write path

A single `upsert_embeddings` API takes `(source_kind, source_id, fields: Vec<(field, text)>)`. For each field:

1. Compute `content_hash = BLAKE3(text)`.
2. If a row already exists with the same `(source_kind, source_id, field, chunk_idx, model_id)` and matching `content_hash`, skip.
3. Otherwise embed and upsert.

This makes "reindex everything" idempotent and cheap when nothing has changed. Re-embedding only happens on real text changes or model changes.

---

## 4. What to Embed for Tasks

### 4.1 Per-field rather than whole-bundle

A task bundle has structured fields with different retrieval value. The design indexes them as separate rows rather than concatenating into a single document:

| Field | Source | Rationale |
|-------|--------|-----------|
| `purpose` | `task.yaml.purpose` | High-density signal; what the task is for |
| `summary` | `task.yaml.summary` | One-line gist; useful for short-query matches |
| `plan` | `plan.md` | Implementation intent; long-form |
| `execution_summary` | `execution-summary.md` | What actually shipped |
| `acceptance_criteria` | `task.yaml.acceptance_criteria[*]` joined | Often the most query-relevant text |
| `comment_<idx>` | `task.yaml.comments[idx].body` | One row per comment; preserves authorship |
| `review_<thread>_msg_<idx>` | review_threads | Decision context lives here |

A single match in a comment surfaces the parent task; the result formatter rolls field-level hits up to task-level results, with the highest-scoring field shown as a snippet ([4_decisions.md ADR-003](./4_decisions.md)).

### 4.2 Chunking long fields

`plan.md` and `execution-summary.md` regularly exceed BGE's 512-token context. The chunker splits on paragraph boundaries with a target of 400 tokens per chunk and a 50-token overlap. Each chunk gets its own row with `chunk_idx = 0, 1, 2, ...`. Queries that match multiple chunks of the same field/task collapse to one result with the best-scoring chunk surfaced.

Token counting uses fastembed-rs's tokenizer for the active model — exact, not heuristic — to keep chunks below the model's actual limit.

### 4.3 Fields *not* embedded

- `task.yaml.id`, `created_at`, `updated_at`, `status`, `dependencies`, `external_refs`: identifiers and structured metadata; FTS5 handles these better.
- `artifacts/**` blobs: out of scope phase 1. Most are binary or large generated content; embedding them is expensive and rarely useful for "find the related task" queries.

---

## 5. Hybrid Retrieval

### 5.1 FTS5 virtual table

A second new table mirrors task content for lexical search:

```sql
CREATE VIRTUAL TABLE tasks_fts USING fts5(
    source_id UNINDEXED,
    field UNINDEXED,
    content,
    tokenize = 'porter unicode61 remove_diacritics 2'
);
```

Populated from the same per-field text as the embedding indexer. FTS5 ships with BM25 ranking built in — no implementation needed beyond the virtual table.

### 5.2 Reciprocal Rank Fusion

Both retrievers run in parallel for a query. Each returns a ranked list of `(source_id, field, chunk_idx)` candidates. RRF combines them:

```
score(c) = Σ over retrievers r of: 1 / (k + rank_r(c))
```

with `k = 60` (the published-paper default that has held up across many evaluations). The fused ranking determines the final result order. RRF is parameter-light, requires no score calibration between retrievers, and consistently beats either retriever alone in the literature.

### 5.3 Why hybrid

Three queries that motivate the choice:

- **"slow embed inference"** — semantic wins; lexical misses tasks titled "BGE latency degraded after Nomic swap."
- **"T20260421-0528"** — lexical wins; semantic returns near-random because the literal token has no semantic neighborhood.
- **"file: orbit-store/src/file/task_store/layout.rs"** — lexical wins; literal path tokens dominate.

Either retriever alone has a failure mode the other doesn't. RRF resolves both at the cost of one extra SQL query per search ([4_decisions.md ADR-004](./4_decisions.md)).

---

## 6. CLI and MCP Surface

### 6.1 CLI

```
orbit semantic search <query> [--limit N] [--field FIELD] [--kind task]
orbit semantic related <task-id> [--limit N]
orbit semantic reindex [--force] [--model MODEL]
orbit semantic stats
```

`search` runs the hybrid pipeline. `related` embeds the target task's `purpose + summary` and runs cosine-only against other tasks (lexical fusion adds noise here). `reindex` rebuilds the `embeddings` rows; `--force` ignores `content_hash` and re-embeds everything. `stats` reports row counts, model distribution, and stale-row count.

### 6.2 MCP tools

- `orbit.semantic.search` — `(query, limit?, kind?, field?)` → ranked results with snippets.
- `orbit.semantic.related` — `(task_id, limit?)` → ranked similar tasks.

Both tools are read-only. Indexing is implicit (on task mutation) or explicit (`orbit semantic reindex`); no MCP write surface.

### 6.3 Result shape

```jsonc
{
  "results": [
    {
      "source_kind": "task",
      "source_id": "T20260421-0528",
      "best_field": "plan",
      "snippet": "...",
      "score": 0.87,
      "score_breakdown": { "rrf": 0.87, "bm25_rank": 4, "cosine_rank": 1 }
    }
  ],
  "model_id": "bge-small-en-v1.5"
}
```

The score breakdown is deliberately exposed: agents can use it to decide whether a hit is "lexical exact match" vs. "semantic neighborhood" and adapt downstream behavior.

---

## 7. Index Maintenance

### 7.1 On-mutation indexing

`task.add` and mutating `task.update` paths emit an `EmbedJob` to a bounded in-process channel after the durable write commits. A worker drains the channel, batches up to 16 jobs at a time, and runs `upsert_embeddings`. Failures log and continue — embedding is not in the critical path of task mutation. Users with no embedder available (airgapped, no model preloaded) see no degradation in core task operations.

### 7.2 Backfill and migration

`orbit semantic reindex` walks the task store and embeds anything not present (or whose `content_hash` differs). A model migration (`--model`) writes new rows under the new `model_id`; the old `model_id` rows can be deleted in a follow-up `orbit semantic prune --model OLD`.

### 7.3 Deletion

`task.delete` cascades to `DELETE FROM embeddings WHERE source_kind = 'task' AND source_id = ?`. Tombstoned tasks (in the v2 task-sync sense, see [docs/design/task-sync/](../task-sync/)) are not embedded.

---

## 8. Concerns & Honest Limitations

### 8.1 First-run model download

The default backend downloads ~30MB on first use. For users behind corporate proxies or in airgapped environments, this is a friction point. Mitigation: clear error message + the airgapped cache-path config in [§2.2](#22-default-implementation-fastembedbackend). This does not eliminate the friction; it just makes it actionable.

### 8.2 Binary size

Adding fastembed-rs and its ONNX Runtime dependency adds ~50MB to the Orbit binary on Linux/macOS. This is significant for a tool that today is single-digit MB. The tradeoff is explicit in [4_decisions.md ADR-001](./4_decisions.md). A future option: ship semantic search as an optional feature flag that produces a separate `orbit-embed` binary, but that complicates the install story and is deferred.

### 8.3 Model-quality calibration is not free

"BGE-small is fine" is a claim, not a measurement. Phase 1 ships with a recall@k evaluation harness (a small labeled set of "find the task about X" queries) so the default is defensible and model swaps can be compared honestly. The harness is itself a design open question ([3_vision.md §1.1](./3_vision.md)) — what's labeled, by whom, and how often it re-runs.

### 8.4 Brute-force scaling ceiling

Cosine over a full table scan stays sub-100ms at ~100K vectors. Phase-2 graph integration will push past that; the schema's forward compatibility with `sqlite-vec` is the planned upgrade path, but `sqlite-vec` is itself a loadable extension that may not be available in every distribution. The decision to revisit storage at phase 2 is in [4_decisions.md ADR-002](./4_decisions.md).

### 8.5 Multilingual content

BGE-small-en is English-tuned. Tasks written primarily in other languages will have weaker semantic recall. fastembed-rs supports multilingual models (e.g. paraphrase-multilingual-MiniLM); the model knob accommodates a swap, but the default ships English-tuned and that's a documented limitation, not a hidden one.

### 8.6 Privacy posture

All embeddings stay local. Task content never leaves the workspace. This is structural — there's no remote API in the loop — but worth stating explicitly because "AI feature" commonly implies "your data is going somewhere," and Orbit's semantic search does not.

---

## Task References

- [T20260510-3] — Design semantic search over task artifacts and graph (v2). The task that produced this folder.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
