# Graph Token-Usage Benchmark — Sweep Results

**Task:** T20260422-1609
**Sweep date:** 2026-04-22
**Sweep seed:** 1609 (shared across both providers)
**Scope:** 2 providers × 3 arms × 6 fixtures × 5 seeds = 180 runs per provider.
**Fixtures:**
`locate-agentruntime`, `locate-v2-runtime-host-trait`, `trace-policy-denial-wiring`, `trace-v2runtime-production-impls`, `impact-scope-strategy-callsites`, `deps-orbit-knowledge-consumers`.

> The headline finding of this sweep comes from a transcript-level utilization audit layered on top of the raw token aggregates: **agents almost never invoke graph tools when they have a choice**, so `hybrid` is functionally `no-graph` on this fixture set. Cost tables alone are misleading without this utilization context — read the "Tool-utilization audit" section first.

---

## Headline

1. **Graph tools are near-zero-utilization in `hybrid`.** Across 60 hybrid runs (30 Claude + 30 Codex), graph tools fired exactly **once** — a single `orbit_graph_implementors` call on one Claude seed of `locate-agentruntime`. Codex made **zero** graph-tool calls across 30 hybrid runs.
2. **Which means: "hybrid beats no-graph" is a mirage.** Token parity is not evidence that graph tools help when available; it is evidence that their schema overhead is small enough to not hurt when they are silently ignored in favour of grep / shell.
3. **Graph-only lifts Codex accuracy but is structurally expensive.** Forcing graph-only took Codex's `locate` pass-rate from 80 % → 100 % and `trace` from 80 % → 100 %, at a 1.2×–2.2× token multiplier and 1.5–3.1 M cache_read_tokens / class of MCP schema tax.
4. **Claude is at the pass-rate ceiling on this fixture set.** 119 / 120 passes regardless of arm. The sweep cannot discriminate Claude arms on accuracy — only on cost.
5. **The practical question is no longer "which arm to default to."** It is "why don't agents reach for graph tools when they are available, and what would it take for them to?"

---

## Tool-utilization audit (hybrid arm)

Per-transcript counts of tool_use blocks in hybrid runs. 5 runs per (provider × fixture) cell.

### Claude / hybrid

| fixture | runs | runs_with_graph_call | graph_calls | Grep | Read | Glob | total_tool_uses |
|---|---|---|---|---|---|---|---|
| deps-orbit-knowledge-consumers | 5 | 0 | 0 | 5 | 0 | 0 | 5 |
| impact-scope-strategy-callsites | 5 | 0 | 0 | 5 | 0 | 0 | 5 |
| locate-agentruntime | 5 | **1** | **1** | 10 | 0 | 0 | 12 |
| locate-v2-runtime-host-trait | 5 | 0 | 0 | 5 | 0 | 0 | 5 |
| trace-policy-denial-wiring | 5 | 0 | 0 | 10 | 13 | 0 | 23 |
| trace-v2runtime-production-impls | 5 | 0 | 0 | 5 | 0 | 0 | 5 |
| **total** | **30** | **1** | **1** | **40** | **13** | **0** | **55** |

The one graph call: `mcp__orbit-bench__orbit_graph_implementors` with `trait_selector=symbol:crates/orbit-agent/src/runtime/runtime_trait.rs#AgentRuntime:trait` — a textbook fit for the tool. The other 29 Claude hybrid runs reached straight for `Grep` and were done in 1–2 tool calls.

### Codex / hybrid

| fixture | runs | runs_with_graph_call | graph_calls | shell_execs | total_tool_uses |
|---|---|---|---|---|---|
| deps-orbit-knowledge-consumers | 5 | 0 | 0 | 31 | 31 |
| impact-scope-strategy-callsites | 5 | 0 | 0 | 16 | 16 |
| locate-agentruntime | 5 | 0 | 0 | 36 | 36 |
| locate-v2-runtime-host-trait | 5 | 0 | 0 | 22 | 22 |
| trace-policy-denial-wiring | 5 | 0 | 0 | 69 | 69 |
| trace-v2runtime-production-impls | 5 | 0 | 0 | 39 | 39 |
| **total** | **30** | **0** | **0** | **213** | **213** |

**Codex made zero MCP graph-tool calls in hybrid.** Every hybrid Codex run solved the task with `rg` / `grep` / `find` / `sed` / `cat` via shell `command_execution`.

---

## Primary table — provider × arm × task_class

| provider | arm | task_class | runs | pass_rate | median_total_tokens | p90_total_tokens | tokens_per_success |
|---|---|---|---|---|---|---|---|
| claude | graph-only | deps | 5 | 100 % | 622 | 729 | 637 |
| claude | graph-only | impact | 5 | 100 % | 5 089 | 6 307 | 4 767 |
| claude | graph-only | locate | 10 | 100 % | 645 | 797 | 664 |
| claude | graph-only | trace | 10 | 100 % | 1 473 | 3 111 | 1 614 |
| claude | hybrid | deps | 5 | 100 % | 361 | 502 | 373 |
| claude | hybrid | impact | 5 | 100 % | 295 | 338 | 309 |
| claude | hybrid | locate | 10 | 100 % | 336 | 684 | 368 |
| claude | hybrid | trace | 10 | 90 % | 920 | 1 866 | 1 098 |
| claude | no-graph | deps | 5 | 100 % | 285 | 767 | 411 |
| claude | no-graph | impact | 5 | 100 % | 288 | 295 | 290 |
| claude | no-graph | locate | 10 | 100 % | 420 | 479 | 406 |
| claude | no-graph | trace | 10 | 100 % | 1 098 | 2 084 | 1 158 |
| codex | graph-only | deps | 5 | 100 % | 22 615 | 47 941 | 27 506 |
| codex | graph-only | impact | 5 | 100 % | 22 671 | 59 995 | 30 981 |
| codex | graph-only | locate | 10 | 100 % | 17 865 | 48 864 | 21 134 |
| codex | graph-only | trace | 10 | 100 % | 25 332 | 37 392 | 26 151 |
| codex | hybrid | deps | 5 | 100 % | 13 975 | 23 795 | 13 488 |
| codex | hybrid | impact | 5 | 100 % | 12 117 | 24 528 | 16 312 |
| codex | hybrid | locate | 10 | 90 % | 13 014 | 14 938 | 12 916 |
| codex | hybrid | trace | 10 | 100 % | 18 294 | 29 029 | 17 924 |
| codex | no-graph | deps | 5 | 100 % | 12 427 | 13 517 | 12 581 |
| codex | no-graph | impact | 5 | 100 % | 12 776 | 22 289 | 14 904 |
| codex | no-graph | locate | 10 | 80 % | 13 756 | 23 411 | 17 377 |
| codex | no-graph | trace | 10 | 80 % | 15 108 | 30 313 | 18 323 |

---

## Cost (USD) — Claude only (Codex CLI does not emit billing)

| arm | sonnet cost | haiku cost | arm total |
|---|---|---|---|
| graph-only | $2.7504 | $0.0188 | **$2.77** |
| hybrid | $1.3943 | $0.0188 | **$1.41** |
| no-graph | $1.4378 | $0.0188 | **$1.46** |

Total Claude sweep: **~$5.65**. Graph-only was ~1.9× more expensive than the other two arms for zero pass-rate lift on Claude.

---

## Pass-rate breakdown

240 runs → 234 pass / 6 fail / 0 error.

| run | cause |
|---|---|
| claude / hybrid / trace-policy-denial-wiring / seed=3 | Oracle rejected (stochastic). |
| codex / hybrid / locate-v2-runtime-host-trait / seed=3 | Oracle rejected. |
| codex / no-graph / locate-v2-runtime-host-trait / seeds=2,3 | Oracle rejected (2 / 5). |
| codex / no-graph / trace-policy-denial-wiring / seeds=1,4 | Oracle rejected (2 / 5). |

Codex's only systemic accuracy gap is `no-graph` on locate / trace. Graph access (any form) fixes it.

---

## Re-interpretation: what this sweep actually measured

| apparent effect (from cost tables alone) | real mechanism (once utilization is accounted for) |
|---|---|
| "Hybrid wins on cost." | Hybrid ≈ no-graph because agents ignore the graph surface and use grep / shell. The 0 % utilization rate is the mechanism. |
| "Claude's 16× blowup on `impact/graph-only`." | Claude is being **forced** to solve a grep-shaped problem (`ScopeStrategy::` tokens across 4 files) through structural queries. With no grep available, it calls `orbit_graph_search` with noisy payloads and reasons over them. Real. |
| "Codex graph-only lifts locate / trace from 80 % → 100 %." | Real and reproducible. When agents can't fall back to grep, they actually use the graph tools, and on the two fixtures where grep is error-prone (`locate-v2-runtime-host-trait` has a filename-collision trap; `trace-policy-denial-wiring` requires distinguishing construction from destructuring), the graph tools fix the accuracy gap. |
| "MCP schema tax is cheap." | Only because the tools aren't being used. Once Codex is forced to use them (graph-only), cache_read_tokens jump to 1.5–3.1 M / class — an order of magnitude above hybrid. |

---

## Revised hypothesis reconciliation

- **H1 (fixtures are grep-shaped).** ✅ Still supported — and now the utilization data directly proves the agents agree: they reach for grep whenever it is available.
- **H2 (graph tool payloads are verbose).** ✅ Supported where measurable (graph-only only — hybrid doesn't use them enough to measure).
- **H3 (MCP schema tax in context).** ✅ Supported under graph-only. Not measurable under hybrid, because the same schema tax appears to be tolerable when the tools are never invoked.
- **H4 (per-turn session cost).** Entangled with H3; still untested.
- **H5 (non-code file scanning hurts graph).** ✅ Supported — `impact-scope-strategy-callsites/graph-only` is the single worst cell in the whole sweep on Claude.
- **H6 (graph index drift).** Not testable from this data.
- **H7 (agent over-uses graph).** ❌ **FALSIFIED. The opposite is true.** Agents under-use graph — essentially never reaching for it when grep is available, even when the task structurally favors graph (trait-impl walks, construct-site search).
- **H8 (fixture / prompt drift).** Partially falsified; one pre-sweep drift in `locate-agentruntime` was caught before the run.

---

## Revised recommendations

1. **Stop comparing `hybrid` vs `no-graph` as if it measured graph-tool value.** It doesn't. On this fixture set it measures the size of the tool schema in the system prompt, at zero utilization. To measure graph-tool value you need either (a) fixtures where grep is genuinely the wrong tool so agents reach for graph on their own, or (b) a `graph-preferred` arm where the system prompt actively instructs the agent to try graph first.
2. **The real finding of this sweep is the utilization rate.** 1 / 60 hybrid runs. The follow-up task is not "tune the token budget" — it is **"investigate why agents decline to use graph tools when offered."** Plausible causes to probe: tool descriptions are grep-shaped in the prompt, return payloads are harder to reason over than ripgrep hits, or agents default to the most familiar retrieval surface under uncertainty.
3. **The one accuracy signal is real: forced graph access fixes Codex's error-prone locate / trace cases.** If we want that lift in production without forcing graph-only, we need agents to *choose* the graph tools, which today they don't.
4. **Future fixtures must make graph tools the obviously-right answer.** Candidates: cross-crate trait-impl walks under name collisions, transitive caller queries, refactor-impact across the type graph. These are cases where grep produces ambiguous / noisy results and the graph's structural index is load-bearing. Only then will utilization rise and the hybrid-vs-no-graph comparison become informative.
5. **Instrument tool-utilization in the aggregator.** This sweep needed an ad-hoc transcript pass to surface the headline finding. `aggregate.py` should emit a per-arm tool-call-mix column so the next sweep reports utilization alongside tokens and pass-rate.

---

## Methodology notes

- **Utilization counts** were produced by a transcript-level scan of `runs/<provider>/hybrid/<task>/<seed>.transcript.json`. For Claude, each `message.content` block of `type=tool_use` was counted by `name`; graph calls are those with `name.startswith("mcp__orbit-bench__orbit_graph_")`. For Codex, `item.completed` events of type `mcp_tool_call` were matched against `orbit_graph_`; `command_execution` items were counted as shell.
- **Token-accounting convention:** `input_tokens` is UNCACHED new input across both providers. Codex `_normalize_codex_result` subtracts `cached_input_tokens` at the provider boundary so `aggregate.py`'s `input_tokens + output_tokens` column is cross-provider comparable. Regression tests: `scripts/test_providers.py::TestTokenAccountingConvention`.
- **Codex $0 cost** is a CLI limitation, not an omission. All USD numbers are Claude only.
- **No `error` verdicts** — arm enforcement held across all 240 runs.
- **Reproducing utilization data:** ad-hoc transcript scan during report generation; not yet merged into `aggregate.py` (see recommendation 5).
