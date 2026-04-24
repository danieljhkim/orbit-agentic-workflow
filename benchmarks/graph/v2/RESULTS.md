# Graph Token-Usage Benchmark — v2 Results

**Task:** T20260423-0507 (v2 fixture design + sweep)
**Sweep date:** 2026-04-23
**Sweep ID:** `20260423-123817-c08a7d` (codex)
**Scope:** 1 provider × 3 arms × 10 fixtures × 3 seeds = 90 codex runs.
**Fixtures:** carried v1's 6 + added 4 grep-hard (`callers-run-deterministic-containers`, `impact-tool-context-struct-literals`, `locate-loopaudit-variants`, `trace-tool-call-event-construct-sites`).

> **Claude was not run in v2.** Two separate claude-sweep attempts hit the subscription usage window before completing, and both attempts' partial artifacts were discarded before the codex-only sweep. Every headline below is **codex-only**. The cross-provider cross-check is deferred to a future round or addendum; see [`METHOD.md`](./METHOD.md) §"Known caveats" #1.

---

## Headline

1. **v1's utilization finding is replicated at 3× the seed count.** Across 30 codex hybrid runs, codex made **zero** graph-tool calls — identical to v1's 0/30 hybrid utilization. The v2 fixtures were designed to be grep-hard specifically to see if agents would start reaching for the graph. They didn't.
2. **`no-graph` now strictly dominates on codex.** 97 % pass-rate, 16 k median total-tokens. `hybrid` matches on both axes because it is functionally `no-graph` (see #1). `graph-only` is **2.6× tokens at the median, 3× at p90, and 7 percentage points lower pass-rate** than `no-graph`.
3. **One new fixture breaks graph-only entirely.** `impact-tool-context-struct-literals` passes 3/3 on both `no-graph` and `hybrid`, and **0/3 on `graph-only`**. Agents make the graph navigation attempts but the assembled answer misses the production construction sites the oracle requires.
4. **The cost widened, not narrowed, between v1 and v2.** v1 recorded 1.2–2.2× token multipliers on graph-only; v2 records 2.6× median and 3× p90. Grep-hard fixtures make structural navigation more expensive (more hops per answer), not cheaper.
5. **The practical question is still "why don't agents use graph tools when offered."** With a harder fixture set that explicitly benefits structural navigation, the utilization rate went from 0/30 (v1) to 0/30 (v2). Fixture redesign alone will not produce the utilization shift.

---

## Tool-utilization audit

Per-arm tool-call counts across all 30 runs per arm. Graph calls here include both MCP invocations (`mcp__orbit-bench__orbit_graph_*`) and shell invocations (`orbit tool run orbit.graph.*`) that `run.py` normalized into the `tool_calls` histogram.

| arm | runs | runs_with_graph_call | graph_calls | shell_execs (`exec_command`) |
|---|---|---|---|---|
| codex / no-graph | 30 | 0 | 0 | 211 |
| codex / hybrid | 30 | **0** | **0** | 209 |
| codex / graph-only | 30 | 30 | 670 | 902 |

Two things stand out:

- **Hybrid is identical to no-graph.** 0 graph calls in 30 runs. Total shell-exec count is nearly the same (211 vs 209). The graph surface exists in the prompt under hybrid but the agent never reaches for it.
- **Graph-only uses graph, but also uses shell twice as much.** 902 shell executions alongside 670 graph calls — the agent is still falling back to shell for most navigation steps even when the arm's intent is graph-first. (The codex graph-only arm enforces through prompt steering, not a hard filter, so this is legal; it is also the pattern that drives the token-cost widening below.)

---

## Primary table — provider × arm × task_class

Verbatim from `aggregate.py`. The `graph_calls` / `graph_call_rate` / `shell_or_fs_calls` columns are derived from `_classify_codex_transcript`, which counts `command_execution` as shell — it does not parse command text for `orbit.graph.*`, so the "graph_calls" column under-counts codex graph activity. See the Tool-utilization audit above for the true per-arm counts.

| provider | arm | task_class | runs | pass_rate | median_total_tokens | p90_total_tokens | tokens_per_success | graph_calls | graph_call_rate | shell_or_fs_calls |
|---|---|---|---|---|---|---|---|---|---|---|
| codex | no-graph | deps | 3 | 100 % | 3 961 | 13 404 | 7 064 | 0 | 0/3 = 0.0 % | 16 |
| codex | no-graph | impact | 6 | 100 % | 9 755 | 21 409 | 10 648 | 0 | 0/6 = 0.0 % | 49 |
| codex | no-graph | locate | 9 | 100 % | 22 815 | 28 094 | 18 732 | 0 | 0/9 = 0.0 % | 52 |
| codex | no-graph | trace | 11 | 100 % | 17 490 | 39 948 | 21 524 | 0 | 0/11 = 0.0 % | 94 |
| codex | hybrid | deps | 3 | 100 % | 5 322 | 14 660 | 8 125 | 0 | 0/3 = 0.0 % | 19 |
| codex | hybrid | impact | 6 | 100 % | 13 373 | 15 460 | 10 670 | 0 | 0/6 = 0.0 % | 37 |
| codex | hybrid | locate | 9 | 89 % | 23 266 | 37 540 | 22 629 | 0 | 0/9 = 0.0 % | 52 |
| codex | hybrid | trace | 12 | 100 % | 16 972 | 64 723 | 23 441 | 0 | 0/12 = 0.0 % | 101 |
| codex | graph-only | deps | 3 | 100 % | 41 412 | 50 737 | 37 577 | 0 | 0/3 = 0.0 % | 71 |
| codex | graph-only | impact | 6 | **50 %** | 35 302 | 86 529 | 87 355 | 0 | 0/6 = 0.0 % | 192 |
| codex | graph-only | locate | 9 | 100 % | 17 673 | 58 227 | 23 744 | 0 | 0/9 = 0.0 % | 142 |
| codex | graph-only | trace | 12 | 100 % | 54 242 | 167 737 | 67 634 | 0 | 0/12 = 0.0 % | 497 |

> Class assignment comes from each fixture YAML's `class:` field, not from the filename prefix. In particular, `callers-run-deterministic-containers` declares `class: trace`, which is why trace contains 12 rows (4 fixtures × 3 seeds) and there is no separate "callers" row.

---

## Cost — not available on codex

Codex's JSON event stream does not expose spend; `total_cost_usd` is reported as `0.0` on every record. The claude sweep that would have produced USD figures did not complete — see [`METHOD.md`](./METHOD.md) §"Known caveats" #1.

Aggregate cache-read-token totals per (arm, class) — the most relevant cost proxy for codex — follow the primary table's shape. Graph-only accumulates roughly an order of magnitude more cache_read_tokens than hybrid/no-graph on the trace class (17.9 M vs 1.5 M), matching v1's 1.5–3.1 M finding and extending it.

---

## Pass-rate breakdown

89 / 90 runs produced an oracle verdict; 1 errored.

| run | cause |
|---|---|
| codex / graph-only / impact-tool-context-struct-literals / seeds 1, 2, 3 | Oracle rejected (consistent — see below). |
| codex / hybrid / locate-v2-runtime-host-trait / seed 3 | Oracle rejected (stochastic, 1 / 3). |
| codex / no-graph / trace-tool-call-event-construct-sites / seed 1 | Codex CLI hit timeout (exit=124). Not an agent correctness issue. |

The 50 % graph-only impact number is entirely driven by **all three seeds** of `impact-tool-context-struct-literals` failing on graph-only. That same fixture passes 3 / 3 on both `no-graph` and `hybrid`, so the fixture itself is solvable — graph-only agents are doing something specifically wrong with the graph-navigation approach. Transcripts show the agent calling `orbit.graph.search`, `orbit.graph.refs`, and `orbit.graph.show` on `ToolContext`, but the returned result set under-includes production struct-literal sites and the answer drops them. Worth a per-transcript audit before v3 design.

---

## New v2 fixtures — pass-rate per arm

| fixture | no-graph | hybrid | graph-only |
|---|---|---|---|
| callers-run-deterministic-containers | 3/3 | 3/3 | 3/3 |
| impact-tool-context-struct-literals | 3/3 | 3/3 | **0/3** |
| locate-loopaudit-variants | 3/3 | 3/3 | 3/3 |
| trace-tool-call-event-construct-sites | 2/3\* | 3/3 | 3/3 |

\* one seed errored via CLI timeout, not an oracle rejection.

Three of the four new fixtures land flat across arms (3/3 everywhere). The one fixture that *does* discriminate, discriminates **against** graph-only — the opposite of the design intent.

---

## Re-interpretation vs v1

| apparent effect (v1) | v2 confirms / overturns |
|---|---|
| "Hybrid ≈ no-graph because agents ignore graph when grep / shell is available." | **Confirmed.** 0/30 hybrid utilization in v2 replicates 0/30 in v1. The grep-hard fixture redesign did not budge the utilization pattern. |
| "Forcing graph-only lifts codex locate/trace from 80 % → 100 %." | **Partly overturned.** In v1, graph-only was a modest accuracy lift at a cost. In v2, graph-only is 90 % vs no-graph's 97 % — a net *loss* on accuracy, with a larger token multiplier. The v1 lift was driven by two specific fixtures (`locate-v2-runtime-host-trait`, `trace-policy-denial-wiring`) where grep was error-prone; those are in v2 too and still pass under no-graph, so the v1 lift was fixture-specific, not a general property. |
| "MCP schema tax is cheap because tools aren't invoked." | **Confirmed for hybrid** (still 0 invocations, still cheap). Graph-only's schema tax is now larger than v1 because the grep-hard fixtures drive more graph hops (trace class: 17.9 M cache_read vs v1's 3.1 M). |
| "Token-parity hybrid vs no-graph is a null result." | **Confirmed.** Hybrid (15 592) ≈ no-graph (16 117) at the median, driven by 0/30 utilization. |
| v1 H7 "agents over-use graph." | **Still falsified.** Opposite direction. |

---

## Hypothesis reconciliation (carried from v1)

- **H1 (fixtures are grep-shaped).** v1 marked this supported; v2 tested it head-on with four intentionally grep-hard fixtures. Result: the grep-hard fixtures did not induce graph-tool use under hybrid (0/30). **Supported: agents prefer grep even when grep is structurally wrong.** What this doesn't establish: whether pointer-only graph reads (v3's hypothesis) would shift the preference.
- **H2 (graph payloads are verbose).** v2 strengthens this with the graph-only cost explosion on trace class (17.9 M cache_read_tokens). Payload volume remains a candidate driver even on the redesigned fixture set.
- **H3 (MCP schema tax in context).** Same pattern as v1.
- **H5 (non-code file scanning hurts graph).** Not directly tested — `.orbitignore` (T20260422-0452) shipped before v2 and is baked into the binary, so v2 cannot measure with-vs-without indexer noise.
- **H7 (agent over-uses graph).** Falsified again; 0/30 under hybrid.

---

## Recommendations

1. **Do not interpret v2 as "fixture quality was the problem."** The grep-hard fixture set shifted graph-only's failure modes but did not shift utilization. The utilization problem is upstream of fixture design.
2. **Codex data alone is enough to support the null-result draft** in [`docs/design/knowledge-graph/5_null_result.md`](../../../docs/design/knowledge-graph/5_null_result.md) — round 2 of the evidence log should append a "v2 round" section using the utilization, cost, and per-fixture data here.
3. **Before v3 (pointer-only graph reads, T20260423-0607) spins up, inspect the three failing seeds of `impact-tool-context-struct-literals` on graph-only.** Either the fixture has an oracle over-specification the graph can't hit, or the graph tool shape is structurally missing a signal the agent needs. This inspection decides whether pointer-only would even address the failure mode.
4. **Do not re-run v2 for claude.** The codex-only data is directional enough for the null-result decision; running the same claude sweep would burn subscription budget without moving the headline (v1 already established Claude's ceiling behaviour, and utilization was 1/60 vs codex's 0/60). If cross-provider evidence is needed at decision time, run claude against v3 directly.
5. **Fold transcript-level graph-via-shell counts into `aggregate.py`.** The aggregator's current `shell_or_fs_calls` column doesn't distinguish `orbit tool run orbit.graph.*` shell executions from other shell execs, making the primary table's "graph_calls" column misleading for codex. The `tool_calls` field on each run record already breaks this out — use it instead of the transcript-level classifier. File as a harness polish task before v3.

---

## Methodology notes

- **Utilization counts** were derived from each run record's `tool_calls` histogram, which `run.py` populates for both providers (claude: raw MCP tool names; codex: both `exec_command` and an `orbit.graph.*` entry when the command invokes one). The aggregator's column under-counts codex graph activity — see recommendation #5.
- **Token-accounting convention:** `input_tokens` is UNCACHED new input. Codex `_normalize_codex_result` subtracts `cached_input_tokens` at the provider boundary so `median_total_tokens` is cross-provider comparable; only codex data here, but the column is still meaningful.
- **Codex cost is $0** by harness constraint (see Cost section above).
- **No oracle rejections on the claude side** because claude did not run.
- **Reproduction:** `make -C benchmarks graph-aggregate GRAPH_VERSION=v2`. The tool-utilization audit is not regenerated by this command; it was produced by a direct `tool_calls` scan.
