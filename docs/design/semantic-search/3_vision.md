# Semantic Search — Vision

**Status:** Draft
**Owner:** claude
**Last updated:** 2026-05-09

This document captures the questions phase 1 deliberately does not answer, the prior work this design builds on or rejects, what is specific to Orbit's situation, and external references for readers who want to dig deeper. The questions in §1 are the most likely sources of post-phase-1 design pressure.

---

## 1. Open Questions

### 1.1 Model-selection evaluation

The design defaults to BGE-small-en-v1.5 because it sits at the favorable end of the published recall/size/latency curve. That justification is published-paper-flavored, not Orbit-specific. To pick a default honestly we need:

- A labeled evaluation set: ~50 queries paired with the task IDs they should find. Hand-curated from real "I was looking for that task about X" episodes.
- A recall@k metric run across at least three candidate models: BGE-small (default), all-MiniLM-L6 (smaller), nomic-embed-text-v1.5 (larger, Matryoshka-truncatable).
- Latency and binary-size measurements on the same hardware.
- A re-run cadence (monthly? on every model release?) and where the harness lives in the repo.

Phase 1 ships the harness; the choice of default may flip if BGE-small underperforms. The harness is a deliverable, not an aspiration.

### 1.2 Airgapped install path

Operators behind corporate proxies or running on networks that don't reach HuggingFace need a way to populate the model cache from a local source. Options:

- **Pre-bundled model in a release tarball.** Bloats the download. Simplest UX.
- **Documented manual placement.** Operator copies model files into `~/.orbit/embed/models/<id>/`. Requires documenting the exact file layout fastembed-rs expects.
- **`orbit semantic fetch-model` subcommand.** Downloads on demand into the cache, can be run from a host with network and the cache moved to the airgapped host.

The third is probably right but isn't decided. Phase 1 ships option 2 (manual placement) and revisits.

### 1.3 Graph corpus scaling beyond brute force

A medium repository's graph holds tens of thousands to hundreds of thousands of symbols. Embedding each at 384d puts the corpus past the comfortable brute-force ceiling. Three candidate paths:

- **`sqlite-vec` loadable extension.** HNSW within SQLite. Schema-compatible with phase 1; the `embedding BLOB` column is the format `sqlite-vec` ingests. Drawback: not always available — distributions and managed environments sometimes refuse loadable SQLite extensions.
- **`hnsw_rs` or `instant-distance` in-process.** Pure-Rust HNSW. Independent of SQLite. Adds an index file alongside the DB, complicates atomic mutations.
- **Shard by `source_kind`.** Brute force stays for `task` (small); a separate index handles `symbol`. Hybrid storage.

Decision deferred to the phase-2 design. The phase-1 schema is intentionally compatible with all three.

### 1.4 Embedding-model upgrade migration

Models improve. The day BGE-medium-en-v2 ships and benchmarks better, the team will want to swap. The design supports this — the `model_id` PK column lets two model generations coexist while reindexing — but the operational story is unspecified:

- Does `orbit semantic reindex --model NEW` block on completion or run in the background?
- Is there a transition period where searches union both models' results, or does the new model become authoritative immediately?
- Who triggers the migration: the user, an upgrade hook, or auto-detection on `orbit upgrade`?

The schema is ready for any answer; the UX is not yet chosen.

### 1.5 Cross-task semantic links

Once embeddings exist, the cheapest possible feature is "show me tasks similar to this one" — already in scope as `orbit semantic related`. Less obvious:

- **Auto-detect duplicate task creation.** Run the new task's `purpose` through cosine before insert; if a high-similarity match exists, warn the user.
- **Auto-suggest dependencies.** A new task's content is highly similar to an in-progress task — should it depend on it?
- **Cluster the backlog.** k-means or HDBSCAN over backlog embeddings to surface theme clusters for triage.

All three are useful, all three are out of scope phase 1, and all three are downstream of the same vector layer. The design intentionally keeps them as orthogonal follow-ups rather than baking any one in.

### 1.6 Review-thread granularity

Phase 1 indexes each review-thread message as a separate row. The alternative — index whole threads as single documents — loses authorship signal but improves recall on multi-message threads where the decision context is spread across replies. Which granularity is more useful for "find me the thread where we decided X" is an empirical question that wants the eval harness from §1.1 to settle.

### 1.7 Whether to embed code in phase 2

Phase 2 will embed graph nodes (modules, symbols). Open question: embed metadata only (`module_path + name + docstring`) or include code bodies? Code-aware embedding models (CodeBERT, voyage-code) exist and outperform general-text models on code retrieval, but they're larger and English-text quality drops. Three options:

- **Metadata only with general model.** Smallest cost, weaker code recall.
- **Metadata only with code model.** Better code recall on the small text we feed it.
- **Code bodies with code model.** Best recall, biggest index, biggest model.

Phase 2 picks one with eval evidence; phase 1 does not pre-commit.

### 1.8 Privacy of telemetry

If the team eventually wants to share an aggregated query-quality signal (e.g. "this query had no good results — log it for the eval set"), how is that opt-in plumbed and what guarantees does it carry? Out of scope phase 1, but worth flagging as a future interaction with [docs/POSITIONING.md](../../POSITIONING.md)'s self-hosted, no-cloud-dependency stance.

---

## 2. Prior Work

### 2.1 Local sentence-embedding tooling

- **fastembed (Python and Rust).** ONNX-based, ships small models out of the box, batched inference. The Rust port is the chosen backend; the Python original is the reference for behavior.
- **sentence-transformers (Python).** The de facto Python ecosystem. Source of most of the published recall/latency benchmarks the design relies on.
- **Candle (HuggingFace, Rust).** Pure-Rust ML framework. Considered as backend; rejected for phase 1 because fastembed-rs is closer to "drop in and run" for a fixed set of embedding models. Candle becomes attractive once the design needs models not in the fastembed-rs catalog.
- **llama.cpp / llama-cpp-rs.** GGUF, runs anything, optional GPU. Considered; rejected for phase 1 as overkill for the small embedding models that do the job. The `Embedder` trait keeps it as a future swap.

### 2.2 Vector storage in SQLite

- **`sqlite-vec` (Alex Garcia).** Successor to `sqlite-vss`. Loadable extension that adds vector indexing — brute force, IVF, HNSW. Schema-compatible with Orbit's BLOB layout. The phase-2 upgrade path.
- **`sqlite-vss`.** Predecessor; deprecated in favor of `sqlite-vec`. Cited only because most older guides reference it.
- **DuckDB VSS extension.** Comparable approach in DuckDB. Out of scope; Orbit uses SQLite throughout.

### 2.3 Hybrid retrieval

- **Reciprocal Rank Fusion (Cormack, Clarke, Büttcher 2009).** The fusion algorithm. Simple, parameter-light, robust across IR benchmarks.
- **ColBERT and late-interaction models.** Higher recall than RRF in research benchmarks but require per-token storage; expensive on a 30K-vector corpus and very expensive on a 300K-vector graph corpus. Out of scope.
- **Vespa, Weaviate, Qdrant, Elasticsearch hybrid.** Production-grade hybrid retrievers in dedicated services. Cited as evidence that hybrid is industry standard, not exotic. None applicable to a single-binary local tool.

### 2.4 Tasks-on-disk semantic search elsewhere

- **GitHub Copilot Workspace.** Cloud-only, embeds code and tickets centrally. Not a precedent for the local-first constraint.
- **Linear's "duplicate detection"** on issue creation. Server-side, presumably embeddings-based. Same constraint mismatch.
- **[`gh-search`-style local fuzzy finders.](https://github.com/junegunn/fzf)** Lexical only. Solves the typo case but not the vocabulary-mismatch case that motivates this design.

The closest precedent in spirit is [Pieces.app](https://pieces.app)'s local snippet search, which uses local embedding models for personal-knowledge retrieval. Different domain, same constraint stack.

---

## 3. What May Be Distinctive

Three properties separate this design from the prior art it draws on.

### 3.1 Single-binary local-only by construction

Every published "hybrid retrieval" production system above runs as a service. Orbit's constraint inverts that: no daemon, no service, no API surface, no auth posture to defend. The design is small enough to fit in-process precisely because the corpus is small (tasks, not the whole web). The `Embedder` trait + brute-force cosine + FTS5 + RRF stack adds up to "hybrid retrieval" but ships as four files in two crates rather than four services.

### 3.2 Forward compatibility with the graph corpus

The schema's `source_kind` discriminator is not future-proofing for its own sake; it commits to a specific phase-2 path where graph symbols join the same vector store under a different `source_kind`. The brute-force ceiling and the `sqlite-vec` upgrade path are sized against that future corpus, not against today's task-only corpus. Most semantic-search-on-tasks projects assume tasks are the whole story; this one explicitly does not.

### 3.3 Failure-mode honesty in the score breakdown

The result shape exposes `bm25_rank` and `cosine_rank` separately on every result. A consumer (especially an agent) can detect "this matched only lexically" or "this matched only semantically" and adjust confidence. Most hybrid systems hide the constituent ranks behind a fused score; surfacing both costs nothing and gives downstream tooling a real signal.

---

## 4. References

### 4.1 Orbit-internal

- [docs/design/CONVENTIONS.md](../CONVENTIONS.md) — folder layout, frontmatter, ADR template.
- [docs/design/knowledge-graph/](../knowledge-graph/) — the corpus phase 2 will index. Phase-1 schema is designed to accommodate it.
- [docs/design/task-sync/](../task-sync/) — relevant for whether embeddings should sync across machines (decision: no, embeddings are derivable from text and stay local).
- [docs/POSITIONING.md](../../POSITIONING.md) — the self-hosted, no-cloud-dependency stance that rules out hosted embedding APIs.
- [README.md](../../../README.md) — single-binary install posture.

### 4.2 External

- **fastembed-rs** — `https://github.com/Anush008/fastembed-rs`. The chosen inference backend.
- **`sqlite-vec`** — `https://github.com/asg017/sqlite-vec`. Phase-2 vector-index upgrade path.
- **BGE embedding family** — `https://huggingface.co/BAAI`. Default model lineage.
- **Reciprocal Rank Fusion paper** — Cormack, Clarke, Büttcher, *Reciprocal Rank Fusion outperforms Condorcet and individual Rank Learning Methods* (SIGIR 2009).
- **Matryoshka Representation Learning** — `https://arxiv.org/abs/2205.13147`. The truncatable-dimension property of nomic-embed and similar models; relevant if dim-vs-quality becomes a phase-2 lever.
- **MTEB (Massive Text Embedding Benchmark)** — `https://huggingface.co/spaces/mteb/leaderboard`. The published recall/latency leaderboard the default-model choice leans on.

---

## Task References

- [T20260510-3] — Design semantic search over task artifacts and graph (v2). The task that produced this folder.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
