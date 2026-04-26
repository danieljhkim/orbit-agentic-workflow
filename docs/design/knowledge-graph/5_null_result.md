# Knowledge Graph — Agent-Facing Tool Surface: Evidence Log

**Status:** Resolved — graph surface retained, provider-dependent.
**Owner:** claude
**Last updated:** 2026-04-24

Evidence log for the question *"do agent-facing `orbit.graph.*` tools earn their token cost against grep + read?"* Populated round-by-round from the `benchmarks/graph*` harness. The v3 round produced a decision against a pre-registered cull threshold; this file is kept as the dated trail that led to it. The filename (`5_null_result.md`) predates the outcome — the question opened as a suspected null result and closed elsewhere. The fossil is intentional.

Numbered `5_` as an extension beyond the required four ([CONVENTIONS.md §1](../CONVENTIONS.md#1-folder-layout-per-feature)): it sits next to `4_decisions.md` as a dated evidence log that feeds ADRs rather than competing with them. The v3 decision (retain the surface) will land as an ADR in `4_decisions.md`; this log records how we got there.

---

## Round v1 — baseline (2026-04-22)

**Sweep:** 240 runs (2 providers × 3 arms × 6 fixtures × 5 seeds). [T20260422-1609]. Full data: [`benchmarks/graph/v1/RESULTS.md`](../../../benchmarks/graph/v1/RESULTS.md).

**Headline:** agents almost never invoke graph tools when they have a choice. In the `hybrid` arm (graph + grep + shell all available), graph tools fired **1 / 60** runs — one Claude seed of `locate-agentruntime`, zero Codex seeds. On the other 59 runs the agent reached straight for `Grep` / `rg`.

**Signals:**
- `hybrid` ≈ `no-graph` on tokens and pass-rate because graph tools are silently ignored when grep is available. Token parity is not evidence the tools help — it is evidence the schema overhead is tolerable when nothing invokes them.
- Forcing `graph-only` lifts Codex pass-rate on two fixtures (80 % → 100 % on `locate` and `trace`) at 1.2×–2.2× tokens and 1.5–3.1 M cache_read_tokens / class of MCP schema tax.
- Claude is at the accuracy ceiling across all arms (119 / 120) — the sweep cannot discriminate Claude arms on correctness, only on cost.
- Hypothesis H7 ("agents over-use graph") is falsified; the opposite is true.

**Limit of v1:** every fixture was solvable by grep + read, so the utilization finding is ambiguous — "agents are picking the right tool" and "graph is the wrong tool" predict the same data. The deeper limit, surfaced only after v3, is that v1's codex cells never offered graph tools as first-class MCP entries — they were reachable only via shell invocation of `orbit tool run orbit.graph.*`. The "0/30 codex hybrid utilization" headline was not a preference measurement; it was a tool-surface asymmetry. v1 did not notice this.

---

## Round v2 — grep-hard fixtures + tool-surface trim (2026-04-23)

**Sweep:** 90 runs (codex only × 3 arms × 10 fixtures × 3 seeds). [T20260423-0507]. Full data: [`benchmarks/graph/v2/RESULTS.md`](../../../benchmarks/graph/v2/RESULTS.md).

**Headline:** v1's utilization finding replicated at 3× the seed count on a harder fixture set. **0 / 30 codex hybrid runs** invoked any graph tool. Adding grep-hard fixtures did not budge the pattern.

**Signals:**
- `no-graph` dominated codex: 97 % pass-rate, 16 k median total-tokens. `hybrid` was functionally identical because 0/30 utilization made it equivalent to `no-graph`.
- `graph-only` was **2.6× tokens at the median, 3× at p90, and 7 pp lower pass-rate** than `no-graph`. The grep-hard fixture redesign made structural navigation more expensive, not cheaper.
- One new fixture (`impact-tool-context-struct-literals`) broke graph-only entirely — 0/3 on graph-only vs 3/3 on the other two arms.
- Claude did not run in v2 (subscription usage window exhausted on two attempts). The round is codex-only.

**What v2 thought it measured and what it actually measured:**

v2 was designed to test whether grep-hard fixtures would induce graph-tool use. The intended finding was "0/30 utilization persists even when grep is structurally wrong, so utilization is upstream of fixture design." That finding held on the numbers — but it held against the same latent harness defect as v1: codex still had no MCP access to the graph tools. The shell-CLI surface was the only path, and the shell selector preferred `rg` every time. Retrospectively, v2 measured the shell selector's `rg`-preference as strongly as v1 did, one round later and at higher seed count.

**Additional finding (surfaced in v3 prep):** 38 / 90 v2 codex runs (42 %) emitted `command_failures`, 157 total. Two large classes were traceable to harness bugs: 45× `attempt to write a readonly database` (sandbox was `read-only`; SQLite WAL needed write access) and 13× `error: unexpected argument '--output' found` (agent inventing CLI flags that don't exist — a failure class MCP schemas prevent entirely). Both classes fixed for v3.

---

## Round v3 — MCP parity for codex (2026-04-24)

**Sweep:** 180 runs (2 providers × 3 arms × 10 fixtures × 3 seeds). Full data: [`benchmarks/graph/v3/RESULTS.md`](../../../benchmarks/graph/v3/RESULTS.md).

**The single experimental change:** codex was given first-class MCP access to the same `orbit_graph_*` backend it had only been able to reach through shell-exec in v1 and v2. Model (`gpt-5.3-codex`), fixtures (10), prompts, sandbox policy, and indexer state were held as close to v2 as the fix allowed. See [`v3/METHOD.md`](../../../benchmarks/graph/v3/METHOD.md) §"Central intervention vs v2".

**Headline:** codex hybrid utilization went from **0 / 30 (v2) → 23 / 30 (v3)**. 79 graph calls against 148 shell calls across the 30 hybrid runs. Same model. Only the access surface moved.

**Signals (codex, v3):**
- Pass rates: no-graph 28/30, graph-only **30/30**, hybrid 29/30. Graph-only was codex's *best* arm on accuracy — the opposite of v2.
- Token cost (median passing run, aggregate across all fixtures): no-graph 22,848, graph-only 24,935 (1.09× aggregate), hybrid 14,907 (0.65× aggregate — hybrid was 35 % cheaper than no-graph because the graph tools let codex skip wasteful shell exploration). The per-fixture distribution is much wider than the aggregate suggests — see §"Disposition" for the per-cell table; one fixture (`impact-tool-context-struct-literals`) hits 12.43×.
- Command failures dropped from 157 → 57. The 45× readonly-DB class and 13× hallucinated-CLI-flag class were eliminated outright. The sandbox widening fixed the first; MCP schemas fixed the second.
- 8 of 10 fixtures had ≥2/3 hybrid graph use. The outlier `impact-scope-strategy-callsites` (0/3 hybrid graph use) is a 4-file grep-ergonomic task where codex correctly picked grep and still passed 3/3. Not a graph-tool loss — a correct selection.

**Signals (claude, v3):**
- Hybrid utilization: **0 / 30.** Tool histogram across 30 hybrid runs: 58 `Grep`, 35 `Read`, 0 graph. Claude had the same MCP graph surface in v1 and v3; neither round lifted hybrid utilization. The claim "MCP access fixes utilization" does not transfer to Claude.
- Pass rates: no-graph 28/30, graph-only 23/30, hybrid 28/30. Graph-only was *worse* than no-graph for Claude.
- Graph-only cost: 1.44× no-graph aggregate; 9 of 10 fixtures exceed the pre-registered 1.3× per-cell threshold (worst: `impact-scope-strategy-callsites` at 27.18×). See §"Disposition" for the table.
- Hybrid cost: 0.79× no-graph (schema caches, never invokes).

**Why codex and claude diverge.** Both had MCP parity in v3. The difference is what the graph tool was competing against in each agent's tool list:

- **Codex hybrid tool list:** one generic `exec_command` + 8 specialized graph tools. Specialized wins against generic; the graph tool is the more-specific match for structural questions.
- **Claude hybrid tool list:** `Read`, `Grep`, `Glob` (three specialized fs primitives) + 8 specialized graph tools. Specialized-vs-specialized competition, and `Grep` carries vastly higher base-rate familiarity from training. The graph tool loses the tool-list contest the same way it loses the shell contest when only `rg` is available.

The mechanism is the same in both cases — the agent picks the most familiar specialized tool for the job — but the outcome is provider-dependent because the providers have different baseline tool surfaces. MCP access changes what's *visible* in the selector; it does not change the selector's priors.

---

## Disposition vs. the pre-registered threshold

v3's `METHOD.md` pre-registered the cull threshold before the sweep ran: keep the agent-facing `orbit_graph_*` MCP surface iff *both*

1. Hybrid utilization ≥ 20 % on at least one provider, **AND**
2. Graph-only median (input + output) tokens ≤ 1.3× the matching no-graph median **for the same (provider × fixture) cell**.

### Utilization (criterion 1)

| provider | hybrid graph-call rate | criterion |
|---|---|---|
| codex | **23/30 = 77 %** | passes |
| claude | 0/30 = 0 % | fails |

Provider-any: **carried by codex.**

### Cost (criterion 2) — per-fixture reading

The pre-registered language is per-cell. An earlier draft of this section reported a single aggregate ratio (1.09× codex, 1.44× claude) computed across all cells — a looser reading than METHOD.md actually specified. The strict per-cell reading:

| fixture | codex go/ng | codex ≤1.3× | claude go/ng | claude ≤1.3× |
|---|---:|:---:|---:|:---:|
| deps-orbit-knowledge-consumers | 0.82× | yes | 3.88× | no |
| locate-agentruntime | 0.44× | yes | 1.80× | no |
| locate-v2-runtime-host-trait | 0.18× | yes | 1.57× | no |
| trace-v2runtime-production-impls | 0.58× | yes | 0.77× | yes |
| trace-policy-denial-wiring | 1.74× | no | 1.46× | no |
| locate-loopaudit-variants | 4.87× | no | 3.88× | no |
| callers-run-deterministic-containers | 1.67× | no | 5.10× | no |
| trace-tool-call-event-construct-sites | 2.08× | no | 2.74× | no |
| impact-scope-strategy-callsites | 1.85× | no | 27.18× | no |
| impact-tool-context-struct-literals | **12.43×** | no | 2.82× | no |
| **per-cell pass count** | **4 / 10** | | **1 / 10** | |

Cost criterion under the literal per-cell reading: **codex passes 4 of 10, claude passes 1 of 10.** Neither provider clears the criterion as a clean per-cell sweep.

The `impact-tool-context-struct-literals` outlier (12.43× on codex) is graph eventually finding the answer through a firehose — one passing run made 17 `pack` calls and burned ~225 k marginal tokens to assemble what no-graph solves in 18 k. Pass-rate parity hides this.

### Decision

**Retain the agent-facing graph surface.** The retention rests primarily on criterion 1 (utilization), which codex carries decisively. The cost criterion is mixed even per-cell; codex passes a plurality of fixtures (4 / 10) and the cells that pass include all four `locate` / `deps` / `trace` cases where graph is the structurally correct tool. The cells that fail cluster on `callers` / `impact` / `trace-construct-sites` — fixtures that either ask graph to do precision work it isn't yet built for (signature-based refs are a superset, not type-resolved truth, per [`2_design.md`](./2_design.md) §"Reference resolution") or expose payload-volume problems on `pack`-heavy navigations.

This is a defensible product call, not a clean benchmark pass. Honesty about the per-cell numbers matters because v4 design must inherit the right unsolved questions, not the wrong ones.

**Provider-dependent caveat to carry forward:** the surface earns its cost on providers whose baseline tool surface is generic (shell-only). It does *not* earn its cost on providers whose baseline already includes specialized fs primitives that overlap in function. For Claude specifically, the v1/v3 data is consistent with "graph tools exist but don't get used" — a latent schema-cache tax Claude pays without return. Whether that tax is worth eating to keep codex happy is a product question, not a benchmark question.

## Implications for `4_decisions.md`

Recorded as [ADR-018](./4_decisions.md#adr-018--retain-agent-facing-orbit_graph_-mcp-surface-provider-dependent-value) — *Retain agent-facing `orbit_graph_*` MCP surface; provider-dependent value* (Accepted, [T20260426-0402]). The ADR carries the four consequences originally drafted in this section:

- The 8-tool `orbit_graph_*` MCP surface stays shipped.
- A diagnostic v4 round is planned to characterise the cost-overshoot fixtures, not to re-litigate the keep/cull decision. Specifically: the `impact-tool-context` firehose, the signature-vs-type-resolved precision gap, and the payload-volume problem on `pack`-heavy navigations.
- Future work on reducing schema-cache overhead and payload size (pointer-only graph reads, [T20260423-0607]) becomes a measured-need item, not a speculative one.
- Future tool-surface decisions for other specialized orbit tooling should examine the same question: is the new tool competing in a shell selector (win), a tool-list selector against a generic alternative (win), or a tool-list selector against a specialized alternative (likely loss)?

## Methodological postscript

Two meta-lessons from this series.

**The measurement caught up late.** v1 and v2's "null result" was partly a measurement artifact: codex never had MCP access in the first two rounds, and the harness did not surface the asymmetry. v3 was the first round where the tool-surface question was asked with each provider on a comparable surface.

**Pre-registered thresholds need pre-registered aggregation.** v3's METHOD specified the cost criterion per-cell ("the same provider × fixture cell") but did not specify *how many cells* must clear 1.3× to count as a pass. An earlier draft of the disposition above quietly substituted a single cross-cell aggregate median, which is the loosest possible reading and is not what was pre-registered. The per-cell table now shows the strict reading. Future rounds must pre-register both the per-cell threshold *and* the cell-count rule (e.g. "≥ 8 of 10 fixtures ≤ 1.3×, no single fixture > 2×") before the sweep runs.

---

## Task References

- **[T20260422-1609]** — v1 graph token-usage sweep (baseline).
- **[T20260423-0507]** — v2 grep-hard fixture design + sweep.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
