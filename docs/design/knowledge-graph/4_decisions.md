# Knowledge Graph — Decisions

ADR-style log of non-obvious design choices behind the knowledge graph. Each entry names the decision, the context that forced it, what we chose, and what we traded away. Entries are append-only and keyed by number; superseded entries are marked, not deleted.

Format for each entry: **Status · Date · Task(s)**, then *Context → Decision → Consequences*. See [1_overview.md](./1_overview.md) and [2_design.md](./2_design.md) for the corresponding implementation; [3_vision.md](./3_vision.md) tracks questions that may trigger future ADRs.

---

## ADR-001 — Content-addressed objects + mutable refs

**Status:** Accepted · 2026-04 · [T20260411-0424]

**Context.** The graph has to survive crashes mid-rebuild, support concurrent reads during a rebuild, and deduplicate unchanged nodes across builds. A single mutable JSON file fails all three.

**Decision.** Adopt a git-style split: immutable content-addressed objects under `objects/<hh>/<hash>.json`, immutable blobs under `blobs/<hh>/<hash>.txt`, immutable per-build index under `index/by-id/<root-graph-hash>.json`, and a mutable branch ref as the only pointer that changes.

**Consequences.**
- Atomic ref swaps via tempfile + rename make interrupted writes safe.
- Object dedup is free because identical content produces identical paths.
- Cost: no GC today — orphan objects accumulate (see [3_vision.md §1.10]).

---

## ADR-002 — Branch-scoped refs over a single shared ref

**Status:** Accepted · 2026-04 · [T20260421-0358]

**Context.** The original layout used `.orbit/knowledge/graph/refs/current.json` — one mutable ref shared across every branch and worktree. The last rebuild won globally. Multi-branch and multi-worktree workflows therefore saw graph reads for the wrong branch, and concurrent rebuilds raced on the single pointer.

**Decision.** Namespace refs by branch: `refs/heads/<branch>.json`. Reads resolve the current git branch; writes fail on detached HEAD rather than invent a label. Reads fall back to the default branch's ref with a stderr warning when the current-branch ref does not yet exist; writes never fall back.

**Consequences.**
- Two worktrees on different branches can rebuild concurrently without corruption.
- A new branch is immediately usable via fallback — no forced rebuild.
- Migration path: legacy `refs/current.json` is moved to `refs/heads/<default>.json` on open.
- Cost: two worktrees on the *same* branch still share a ref (see [2_design.md §6.5]).

---

## ADR-003 — Tree-sitter extractors over an LSP backend

**Status:** Accepted · 2026-04

**Context.** Reference resolution is strongest via a language server, but LSPs are stateful long-running processes tuned for interactive UX. Agent tools want bulk, structured, token-budgeted output and low lifecycle overhead.

**Decision.** Use tree-sitter grammars with per-language extractors (`rust`, `python`, `go`, `java`, `javascript`) producing structural symbols only. Defer cross-file reference resolution indefinitely. See [3_vision.md §1.1] for the open question of re-introducing LSP as a pluggable backend.

**Consequences.**
- Fast, deterministic extraction with no per-query process lifecycle.
- `find_references`, `callers`, `implementors` ([T20260412-0645-3]) are signature-matched, not type-resolved — a superset of the truth.
- Cost: extractor maintenance scales with languages supported.

---

## ADR-004 — Shell out to the `git` CLI instead of an in-process library

**Status:** Accepted · 2026-04 · [T20260421-0528]

**Context.** The history walker diffs every new commit against its first parent, parses unified diffs, and resolves trees. An in-process git library (`gix`, `git2`) would avoid per-commit fork cost.

**Decision.** Shell out to `git` via `orbit_common::utility::git::run_git`. No in-process git dependency.

**Consequences.**
- Behavior matches what a user sees on the command line — trivially reproducible.
- No linked library adds build surface or ABI risk.
- Cost: per-commit fork overhead. Tolerable at current repo sizes; revisit if it shows up in refresh profiles.

---

## ADR-005 — Working graph is in-memory, not persistent

**Status:** Accepted (with open question) · 2026-04 · [T20260411-0424]

**Context.** Activities mutate the graph as they edit code. Those mutations must not perturb the persisted store mid-turn (cache stability, concurrent-reader safety). Two implementations are plausible: in-memory overlay vs. per-activity disk staging.

**Decision.** Keep the working graph in memory for the duration of an activity. Persist at activity boundaries only.

**Consequences.**
- Branch ref stays byte-stable inside an activity — queries are reproducible.
- Zero disk churn for reads-only activities.
- Cost: a crashed long activity loses its staging. Recovery = rerun. See [3_vision.md §1.3].

---

## ADR-006 — Hunk-to-symbol attribution by line-range overlap only

**Status:** Accepted · 2026-04 · [T20260421-0528]

**Context.** `git log --follow` chases renames through history but at non-trivial per-hop cost. Hunk coordinates have to be re-mapped after every rename hop. At commit volumes typical of this repo, follow mode compounds into minutes of extra walker time.

**Decision.** Map hunks to leaves by line-range overlap against the symbol's span *at the commit's tree*. Do not chase renames. A symbol moved across files gets attribution from post-move commits only.

**Consequences.**
- Walker cost is predictable and linear in commits, not in rename hops.
- Older attribution is lost when a file is renamed or heavily reformatted.
- Pure deletions credit the insertion-point symbol — approximation on purpose.
- See [2_design.md §6.3] for the full caveat.

---

## ADR-007 — Task-ID attribution is a flat union, not state-aware

**Status:** Accepted (with open question) · 2026-04 · [T20260421-0528]

**Context.** A leaf touched by a reverted task and by a shipped task currently carries both IDs with no distinction. Consumers that want "which change is live" signal have to join against task status externally.

**Decision.** Keep `task_ids` a flat union at the graph layer. Do not embed lifecycle state in the graph.

**Consequences.**
- Graph stays independent of task-state evolution (which changes more often than code structure).
- Consumers wanting shipped-only signal take the join hit.
- Open question: [3_vision.md §1.11] may reopen this if the external join proves too painful.

---

## ADR-008 — File-based lock store in `orbit-knowledge`, no standalone `orbit-lock` crate

**Status:** Accepted · 2026-04 · [T20260411-0424], [T20260417-0301-2]

**Context.** An earlier prototype factored graph-node locks into a standalone `orbit-lock` crate. The crate added a dependency edge without buying reuse — no consumer other than `orbit-knowledge` ever imported it.

**Decision.** Keep a file-based shared lock store inside `orbit-knowledge::lock`. Remove the `orbit-lock` crate.

**Consequences.**
- One fewer crate in the architecture diagram; simpler dependency graph.
- Locks remain file-backed and process-shareable, matching the content-addressed store's on-disk model.
- Hardened incrementally — [T20260417-0301-2] closed holes around concurrent write paths.

---

## ADR-009 — Debounced, single-flighted refresh

**Status:** Accepted · 2026-04 · [T20260417-0307], [T20260416-0719], [T20260417-0639]

**Context.** Every read can trigger `ensure_fresh`. Without coordination, a dirty worktree plus many quick reads would stack rebuilds, and concurrent callers would duplicate work.

**Decision.** Guard rebuilds with a `flock` on `refresh.lock`. Debounce dirty-worktree rebuilds against a fingerprint + timestamp in `refresh_state.json` (default 5s, `ORBIT_KNOWLEDGE_REFRESH_DEBOUNCE_SECS`). Concurrent callers wait briefly for the in-flight rebuild rather than starting their own.

**Consequences.**
- Steady-state read cost on a dirty worktree is one rebuild per debounce window, not one per read.
- Corrupt-store recovery path ([T20260416-0719]) lives in the same critical section.
- Cost: the first reader after a change pays full rebuild latency; subsequent readers ride the cache.
