# Semantic Search — Decisions

**Status:** Draft
**Owner:** claude
**Last updated:** 2026-05-09

ADR-style log of non-obvious semantic-search decisions. Each entry names the pressure, the choice, and the tradeoff. Entries are append-only and keyed by number; superseded entries are marked, not deleted.

Format for each entry: **Status · Date · Task(s)**, then *Context → Decision → Consequences*. Every ADR names at least one cost. ADRs in this file carry status `Proposed` until the implementing task ships; they flip to `Accepted` with the implementing task ID at that point.

---

## ADR-001 — fastembed-rs ONNX backend over Candle, llama.cpp, or external ollama

**Status:** Proposed · 2026-05 · [T20260510-3]

**Context.** Local embedding inference has four plausible backends:

| Backend | Profile |
|---------|---------|
| **fastembed-rs** | Pure Rust crate wrapping ONNX Runtime; ships a small set of well-known sentence-embedding models (BGE, MiniLM, Nomic, mxbai); CPU-only fine; batch-friendly. |
| **Candle** | Pure-Rust ML framework from HuggingFace; broader model support; more code to integrate; less plug-and-play for embeddings specifically. |
| **llama-cpp-rs** | Bindings to llama.cpp; GGUF format; runs anything from tiny embedding models to large LLMs; optional GPU; C++ build dependency. |
| **External ollama or similar daemon** | Zero binary cost in Orbit; outsources inference; requires the user to install and run a separate process. |

The hard constraint is Orbit's single-binary install posture ([docs/POSITIONING.md](../../POSITIONING.md), [README.md](../../../README.md)). A required external daemon contradicts that doctrine. Within in-process options, fastembed-rs covers the embedding-model use case directly; Candle is more general but requires more Orbit-side code; llama-cpp-rs is overkill and adds a C++ build dependency that complicates Orbit's release pipeline.

**Decision.** Phase 1 uses fastembed-rs as the default backend, accessed through an `Embedder` trait in a new `orbit-embed` crate. The default model is BGE-small-en-v1.5 (384 dim, ~30MB). Other backends can be added later as additional `Embedder` implementations without changing storage or retrieval. Reject external ollama: violates single-binary doctrine. Reject llama-cpp-rs: C++ build dependency and binary-size cost outweigh its flexibility for embedding-only work. Reject Candle as default: more integration work for less out-of-the-box behavior; remains a viable swap-in.

**Consequences.**
- Single-binary install posture preserved.
- No daemon to install, no port to manage, no auth surface to defend.
- Model swap is a configuration knob, not a code change, because the trait is small.
- Cost: fastembed-rs and ONNX Runtime add ~50MB to the Orbit binary on Linux/macOS — significant for a tool that today is single-digit MB. Operators who never use semantic search still pay the binary cost unless we later split into a feature flag (deferred).

---

## ADR-002 — Brute-force cosine over SQLite BLOBs; `sqlite-vec` reserved as phase-2 upgrade

**Status:** Proposed · 2026-05 · [T20260510-3]

**Context.** Vector storage and retrieval has three plausible shapes:

1. **Brute-force cosine in Rust over SQLite BLOBs.** No new dependency. Linear scan per query.
2. **`sqlite-vec` loadable extension.** HNSW-indexed nearest-neighbor inside SQLite. Same on-disk format as (1). Adds a runtime extension load.
3. **Standalone vector DB** (Qdrant, LanceDB, ChromaDB). Production-grade. Adds a binary dependency or sidecar.

At phase-1 scale — tasks-only, low thousands of artifacts × small number of fields per task = tens of thousands of vectors at 384d — brute-force cosine is sub-100ms on a modern laptop and zero new dependencies. `sqlite-vec` is the right answer once the corpus crosses ~100K vectors; that crossing happens with phase-2 graph integration, not phase 1. Standalone vector DBs are inappropriate for an embedded local tool.

A subtle point: the choice of `embedding BLOB` storage format in (1) is forward-compatible with `sqlite-vec`. Upgrading is a CREATE VIRTUAL TABLE plus an INSERT … SELECT, not a schema rewrite.

**Decision.** Phase 1 implements brute-force cosine in Rust over `embeddings.embedding` BLOBs. The schema preserves forward compatibility with `sqlite-vec` (same BLOB layout, same `dim` and `model_id` columns). Phase 2's graph corpus revisits storage as a separate ADR; if `sqlite-vec` is the right call at that point, it's an additive change, not a migration.

**Consequences.**
- Zero new runtime dependencies in phase 1.
- Schema and on-disk layout are stable across the phase-1/phase-2 boundary.
- Query latency is acceptable until the corpus crosses ~100K vectors.
- Cost: brute force scans every row every query. For a stable phase-1 corpus that's fine, but it means we can't ship "semantic search across the entire repository graph" without revisiting storage. The decision deliberately scopes phase 1 to where brute force is comfortable, and pays the upgrade cost later when there is operational evidence to size against.

---

## ADR-003 — Per-field embeddings with chunked overflow, not whole-bundle concatenation

**Status:** Proposed · 2026-05 · [T20260510-3]

**Context.** A task bundle has structurally distinct fields (purpose, summary, plan, acceptance criteria, comments, review threads) of widely varying length. Two embedding strategies exist:

- **Concatenate everything into one document and embed once.** Simplest; one row per task. Loses precision because a strong match in `purpose` is averaged with weak signal from twenty unrelated comments. Long bundles routinely exceed BGE-small's 512-token context, forcing arbitrary truncation.
- **Per-field embeddings, with long fields chunked at paragraph boundaries.** Multiple rows per task. Best-matching field surfaces in the result. Chunking handles the context-window limit cleanly.

The cost of per-field is mostly storage (~5–20× rows per task) and indexing CPU. At BGE-small's 384d, even a generous 20 rows × 10K tasks = 200K rows × 1.5KB = 300MB. Fits comfortably in SQLite, comfortable for brute force at this scale.

**Decision.** Phase 1 indexes one row per `(task_id, field, chunk_idx)`. Result formatting collapses multiple field hits on the same task to a single result with the highest-scoring field surfaced as the snippet. Long fields (`plan.md`, `execution-summary.md`) are split at paragraph boundaries with a target of 400 tokens per chunk and 50-token overlap.

**Consequences.**
- Result snippets point to the actual field that matched, which makes the answer interpretable to users and agents.
- Comments and review messages become independently findable, which directly addresses the "decisions buried in long threads" failure mode in [1_overview.md §1](./1_overview.md).
- Schema's `field` column carries the discriminator without a separate table.
- Cost: 5–20× more rows per task, more storage, more indexing CPU. At phase-1 scale the cost is unproblematic; at much larger scales the per-field strategy may need revisiting alongside the storage upgrade in ADR-002.

---

## ADR-004 — Hybrid retrieval (FTS5 BM25 + cosine, fused via RRF) from day one

**Status:** Proposed · 2026-05 · [T20260510-3]

**Context.** Three retrieval strategies were on the table:

- **Semantic only.** Strong on vocabulary mismatch; weak on literal-identifier queries (function names, error codes, task IDs, file paths). Ignores SQLite's already-shipped FTS5 BM25 capability.
- **Lexical only.** The status quo without this design. Fast, free, well-understood. Cannot find tasks whose vocabulary doesn't match the query.
- **Hybrid: BM25 + cosine, fused via Reciprocal Rank Fusion.** Both retrievers run in parallel; ranks combine without score calibration. Published research consistently shows hybrid beats either alone across information-retrieval benchmarks.

The third option costs one extra SQL query per search and ~30 lines of fusion code. SQLite ships FTS5 with BM25 built in, so the lexical side is essentially free — the implementation is `CREATE VIRTUAL TABLE tasks_fts USING fts5(...)`. Picking semantic-only would be a deliberate choice to fail on literal-identifier queries, which agents query frequently.

A weighted combination (e.g. `0.6 * cosine_score + 0.4 * bm25_score`) was considered as an alternative fusion. Rejected because BM25 and cosine produce scores on incommensurable scales, weights become a tuning knob with no obvious right answer, and RRF demonstrates equal or better quality without the calibration burden.

**Decision.** Phase 1 ships hybrid retrieval. Both retrievers run on every `search` query. RRF (k=60) fuses the rankings. Score breakdown (`bm25_rank`, `cosine_rank`) is exposed in result payloads so consumers can detect which retriever drove a given hit. `related` (similar-task discovery) is cosine-only because lexical similarity adds noise for that use case.

**Consequences.**
- Literal-identifier queries (task IDs, function names, file paths) match correctly.
- Vocabulary-mismatch queries match correctly.
- Score breakdown gives agents a real signal for confidence calibration without exposing raw incommensurable scores.
- Cost: every `search` runs two SQL queries instead of one and computes one extra fusion pass. At phase-1 latency budgets (target <200ms p95) this is unproblematic, but it doubles the per-query work versus a single-retriever design and that overhead is paid even on queries where one retriever would have been enough.

---

## Task References

- [T20260510-3] — Design semantic search over task artifacts and graph (v2). The task that produced this folder.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
