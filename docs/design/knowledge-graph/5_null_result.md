# Knowledge Graph — Null Result Evidence Log

**Status:** Draft
**Owner:** claude
**Last updated:** 2026-04-23

Evidence log for the question *"do agent-facing `orbit.graph.*` tools earn their token cost against grep + read?"* Populated round-by-round from the `benchmarks/graph*` harness. If successive rounds show no edge, this file's conclusion is promoted into a new ADR in [`4_decisions.md`](./4_decisions.md) that retires the agent-facing graph surface (the `orbit-knowledge` crate stays — its indexer has infra uses independent of agent navigation).

Numbered `5_` as an extension beyond the required four ([CONVENTIONS.md §1](../CONVENTIONS.md#1-folder-layout-per-feature)): it sits next to `4_decisions.md` as a dated evidence log that feeds ADRs rather than competing with them. When the decision is drawn, this file is archived and the ADR in `4_decisions.md` becomes the canonical record.

---

## Round v1 — baseline (2026-04-22)

**Sweep:** 240 runs (2 providers × 3 arms × 6 fixtures × 5 seeds). [T20260422-1609]. Full data: [`benchmarks/graph/v1/RESULTS.md`](../../../benchmarks/graph/v1/RESULTS.md).

**Headline:** agents almost never invoke graph tools when they have a choice. In the `hybrid` arm (graph + grep + shell all available), graph tools fired **1 / 60** runs — one Claude seed of `locate-agentruntime`, zero Codex seeds. On the other 59 runs the agent reached straight for `Grep` / `rg`.

**Signals:**
- `hybrid` ≈ `no-graph` on tokens and pass-rate because graph tools are silently ignored when grep is available. Token parity is not evidence the tools help — it is evidence the schema overhead is tolerable when nothing invokes them.
- Forcing `graph-only` lifts Codex pass-rate on two fixtures (80 % → 100 % on `locate` and `trace`) at 1.2×–2.2× tokens and 1.5–3.1 M cache_read_tokens / class of MCP schema tax.
- Claude is at the accuracy ceiling across all arms (119 / 120) — the sweep cannot discriminate Claude arms on correctness, only on cost.
- Hypothesis H7 ("agents over-use graph") is falsified; the opposite is true.

**Limit of v1:** every fixture was solvable by grep + read, so the utilization finding is ambiguous — "agents are picking the right tool" and "graph is the wrong tool" predict the same data. v2 is designed to resolve this with fixtures where grep is structurally wrong (construct-vs-destructure, cross-crate caller walks, multi-variant enumeration) before a null-result decision is drawn.

---

## Task References

- **[T20260422-1609]** — v1 graph token-usage sweep (baseline).

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
