# AGENTS — orbit-agent (Python)

This file provides context for agents working on the `orbit-agent` Python CLI app.

---

## What is orbit-agent?

`orbit-agent` is the **default knowledge builder** for Orbit. It scans a codebase, extracts structure and semantics, and produces deterministic artifacts under `.orbit/knowledge/`.

The core idea: instead of agents re-exploring a repo every time, `orbit-agent` **pre-compiles code understanding into persistent, versioned artifacts** that Orbit consumes at runtime. Think `cargo build` for code knowledge.

---

## Relationship to the Rust Codebase

The Orbit Rust workspace lives at `orbit/` and contains 10 crates:

```
orbit-types → orbit-policy, orbit-exec → orbit-tools → orbit-store, orbit-agent → orbit-engine → orbit-core → orbit-cli
```

The Rust `orbit-agent` crate (`orbit/orbit-agent/`) currently handles the **agent runtime** — provider abstraction (Claude, Codex, mock) via the `AgentRuntime` trait.

This Python `orbit-agent` handles **knowledge building**. Its optional LLM-backed summarization stages use a temporary provider boundary under `orbit_agent.runtime.agent`, but deterministic graph/bootstrap/context code should not import or initialize provider implementations. Long term, provider adapters should move behind a reusable Orbit SDK/runtime that both knowledge generation and Orbit-owned execution flows can consume.

---

## Pipeline

```
scan repo → hash files → detect changes → summarize files → generate architecture → write artifacts
```

Planned module layout:

```
orbit-agent/
  main.py
  pipeline/
    scan.py       # file walking
    hash.py       # content hashing (sha256)
    summarize.py  # file-level LLM summaries
    architecture.py  # system-level LLM summaries
  runtime/
    agent/        # temporary provider runtime boundary
  schemas/
    knowledge.py  # pydantic models for schema validation
```

Key libraries: `pathlib`, `hashlib`, `pydantic`, `tiktoken`, LLM client (OpenAI / local).

---

## Output: Knowledge Artifacts

All output is written to `.orbit/knowledge/` with this structure:

```
.orbit/knowledge/
  manifest.json       # entry point — schema version, timestamp, repo root
  architecture.json   # high-level components, flows, dependencies
  files/
    <hash>.json       # per-file summary, symbols, imports/exports
```

### Schema (v1) — defined in `schema.md`

**manifest.json**: `schemaVersion`, `generated_at`, `repo_root`, `artifacts` pointers.

**architecture.json**: `summary` (string), `components` (name/role/depends_on), `key_flows` (name/description/steps).

**files/\<hash\>.json**: `path`, `hash`, `language`, `summary`, `symbols` (name/kind/signature/description), `imports`, `exports`, `metadata` (size_bytes, last_modified).

Optional: `.orbit/cache/hashes.json` for incremental rebuild support.

---

## How Orbit Consumes Knowledge

Orbit never explores the codebase directly. At runtime:

1. Load `manifest.json` and validate schema version
2. Load `architecture.json`
3. Score file summaries against the task query (keyword + structure matching)
4. Select top-K files within token budget
5. Inject architecture summary + selected file summaries into prompt

### Retrieval Scoring (v1)

Per file summary: +2 keyword match in `path`, +3 in `symbols[].name`, +1 in `summary`, +1 in `imports`/`exports`. Tie-break: shorter path depth > smaller size > lexicographic. Budget enforced by token estimation (chars/4).

---

## CLI

```bash
orbit-agent build                  # full scan, rebuild all artifacts
orbit-agent update                 # incremental — only reprocess changed files
orbit-agent build --repo . --output .orbit/knowledge --incremental
```

---

## Design Principles

- **Deterministic** — no randomness in output
- **Incremental** — only process what changed
- **Auditable** — diff-friendly JSON
- **Minimal** — avoid over-modeling
- **Decoupled** — schema-first; builder is replaceable (any tool can produce compatible artifacts)

---

## What orbit-agent Does NOT Do

- Execute tasks
- Make runtime decisions
- Interact with Orbit workflows directly
- Replace language servers or provide full AST fidelity

---

## Future Work

- Embedding-based retrieval
- Graph construction
- Cross-repo linking
- Symbol-level indexing
- Language-specific parsers (AST)
- Alternative/pluggable knowledge builders
