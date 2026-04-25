# Graph Token-Usage Benchmark - v4 Results

**Status:** Complete for Codex and Claude. Codex graph-only rerun post-fix on 2026-04-25 to remove the tool-fix confound.
**Sweep dates:** 2026-04-24 to 2026-04-25
**Codex sweep IDs:** `20260424-230632-f36f84` (pre-fix `no-graph`, `graph-only`), `20260425-001959-842b8c` (pre-fix `hybrid`), `20260425-115117-21a938` (post-fix `graph-only`)
**Claude sweep IDs:** `20260425-013339-cfac6a` (`no-graph`, `graph-only`), `20260425-012511-8881cb` (`hybrid`)
**Codex sweep seeds:** `928176111` (pre-fix `no-graph`, `graph-only`), `583229300` (pre-fix `hybrid`), `142643867` (post-fix `graph-only`)
**Claude sweep seeds:** `152346771` (`no-graph`, `graph-only`), `88160319` (`hybrid`)
**Harness SHAs:** Codex pre-fix `b0ce189e7053409c8754865bd154cd20e1de66a6`; Claude post-fix run / Codex post-fix graph-only rerun `56a9c07b64479360f9a64ca94b40721f76226014` (post-fix branch contains both `T20260425-0729` and `T20260425-0739`).
**Scope:** 228 cells total. Each provider ran `no-graph` 12 fixtures x 3 seeds, `graph-only` 12 fixtures x 3 seeds (Codex's was run twice — pre- and post-fix), and `hybrid` 8 fixtures x 3 seeds. No errored cells.

**Comparison caveat:** Codex `no-graph` and `hybrid` are pre-fix; Codex `graph-only` was rerun post-fix and is the canonical Codex-vs-Claude comparison row. Pre-fix Codex `graph-only` artifacts are preserved at `benchmarks/graph/v4/_archive/codex-graph-only-pre-fix-T20260425-0739/` and remain available as a "what the bug looked like" reference. Codex `no-graph` was not rerun (unaffected by either fix). Codex `hybrid` was not rerun (passed 24/24 pre-fix; the only delta would be 3 failed graph calls becoming 0).

---

## Headline

1. **Codex pre-fix:** `no-graph` passed 36/36, `graph-only` passed 30/36, and `hybrid` passed 24/24. All six Codex graph-only failures came from `module-surface-orbit-mcp` and `reverse-export-orbit-error`, the two fixtures most exposed to the `T20260425-0739` re-export bug.
2. **Codex post-fix (graph-only rerun):** `graph-only` passed **36/36** (was 30/36), median **12,928 tokens** (was 15,462), failed graph calls dropped to **4** (was 25). The two formerly-failing fixtures both flipped to 3/3 — confirming `T20260425-0739` was the right diagnosis. Schema-coercion churn was eliminated by `T20260425-0729`.
3. **Claude post-fix:** `graph-only` passed 36/36, `hybrid` passed 24/24, and `no-graph` passed 34/36. The two Claude no-graph failures were both `const-value-extraction` runs that omitted `V2_TOOL_WILDCARD_ROOTS`.
4. **The fix is symmetric across providers:** post-fix, both Codex and Claude graph-only pass `reverse-export-orbit-error` and `module-surface-orbit-mcp` 3/3. The Codex-vs-Claude comparison is now clean — no tool-bug confound.
5. **Graph-only accuracy improved post-fix, but cost shifted unevenly:** the two formerly-failing fixtures became expensive-passes (Codex `reverse-export-orbit-error` 122,948 tokens vs 13,912 no-graph; `module-surface-orbit-mcp` 21,885 vs 5,886). Other fixtures got cheaper because schema-coercion retries are gone. Net: median dropped, p90 stayed high.
6. **Hybrid remains the practical operating mode, but providers route differently:** Codex hybrid used graph in 11/24 runs and passed all seeds. Claude hybrid used graph in only 3/24 runs, all on `deps-downstream-orbit-knowledge`, and passed the rest via shell/source fallback.
7. **The remaining graph work is payload shaping and selector ergonomics:** post-fix Codex still hit 4 failed graph calls (2 empty-query searches, 2 unprefixed selectors). Claude post-fix hit 9 (nested-list/invalid-selector shapes). Both classes are recoverable but waste tokens on retry cycles.

---

## Arm Summary

Token totals are `input_tokens + output_tokens`, matching the aggregator's marginal-token convention. Cached read tokens and Claude USD cost are reported separately by the raw records, but not included in the median-token columns.

| provider | arm | runs | pass | median_total_tokens | p90_total_tokens | graph_call_rate | graph_calls | failed_graph_calls | shell_or_fs_calls |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| claude | no-graph | 36 | 34/36 | 713 | 3159 | 0/36 | 0 | 0 | 154 |
| claude | graph-only | 36 | 36/36 | 2330 | 6866 | 36/36 | 436 | 9 | 0 |
| claude | hybrid | 24 | 24/24 | 663 | 2449 | 3/24 | 3 | 0 | 45 |
| codex | no-graph | 36 | 36/36 | 11446 | 27792 | 0/36 | 0 | 0 | 197 |
| codex | graph-only (pre-fix) | 36 | 30/36 | 15462 | 64877 | 36/36 | 334 | 25 | 0 |
| codex | graph-only (post-fix) | 36 | 36/36 | 12928 | 71774 | 36/36 | 438 | 4 | 0 |
| codex | hybrid | 24 | 24/24 | 3900 | 11048 | 11/24 | 40 | 3 | 61 |

Hybrid only ran on the 8 graph-strength and precision-gap fixtures, per `METHOD.md`. On that same 24-run subset:

| provider | no-graph median | graph-only median | hybrid median |
|---|---:|---:|---:|
| claude | 491 | 1541 | 663 |
| codex | 11446 | 15114 | 3900 |

---

## Primary Aggregate

Verbatim from:

```bash
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/aggregate.py \
  --runs benchmarks/graph/v4/runs \
  --tasks benchmarks/graph/v4/tasks
```

| provider | arm | task_class | runs | pass_rate | median_total_tokens | p90_total_tokens | tokens_per_success | graph_calls | graph_call_rate | shell_or_fs_calls |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| claude | graph-only | graph-strength | 12 | 100% | 1310 | 3944 | 1871 | 77 | 12/12 = 100.0% | 0 |
| claude | graph-only | payload-volume | 6 | 100% | 5128 | 11584 | 5636 | 201 | 6/6 = 100.0% | 0 |
| claude | graph-only | precision-gap | 12 | 100% | 1854 | 4538 | 2252 | 90 | 12/12 = 100.0% | 0 |
| claude | graph-only | selector-ambiguity | 6 | 100% | 3960 | 7645 | 4308 | 68 | 6/6 = 100.0% | 0 |
| claude | hybrid | graph-strength | 12 | 100% | 669 | 1842 | 818 | 3 | 3/12 = 25.0% | 12 |
| claude | hybrid | precision-gap | 12 | 100% | 468 | 2887 | 935 | 0 | 0/12 = 0.0% | 33 |
| claude | no-graph | graph-strength | 12 | 100% | 536 | 1841 | 737 | 0 | 0/12 = 0.0% | 31 |
| claude | no-graph | payload-volume | 6 | 67% | 1517 | 2508 | 2207 | 0 | 0/6 = 0.0% | 34 |
| claude | no-graph | precision-gap | 12 | 100% | 343 | 3052 | 874 | 0 | 0/12 = 0.0% | 30 |
| claude | no-graph | selector-ambiguity | 6 | 100% | 2677 | 5296 | 3188 | 0 | 0/6 = 0.0% | 59 |
| codex | graph-only (pre-fix) | graph-strength | 12 | 75% | 9352 | 78425 | 33907 | 115 | 12/12 = 100.0% | 0 |
| codex | graph-only (pre-fix) | payload-volume | 6 | 100% | 16855 | 101587 | 29067 | 57 | 6/6 = 100.0% | 0 |
| codex | graph-only (pre-fix) | precision-gap | 12 | 100% | 15450 | 24307 | 14631 | 91 | 12/12 = 100.0% | 0 |
| codex | graph-only (pre-fix) | selector-ambiguity | 6 | 50% | 21961 | 37822 | 44688 | 71 | 6/6 = 100.0% | 0 |
| codex | graph-only (post-fix) | graph-strength | 12 | 100% | 8822 | 139043 | 36989 | 153 | 12/12 = 100.0% | 0 |
| codex | graph-only (post-fix) | payload-volume | 6 | 100% | 41726 | 76033 | 41984 | 106 | 6/6 = 100.0% | 0 |
| codex | graph-only (post-fix) | precision-gap | 12 | 100% | 8813 | 35578 | 12930 | 107 | 12/12 = 100.0% | 0 |
| codex | graph-only (post-fix) | selector-ambiguity | 6 | 100% | 26106 | 32152 | 24042 | 72 | 6/6 = 100.0% | 0 |
| codex | hybrid | graph-strength | 12 | 100% | 4232 | 14417 | 5394 | 26 | 9/12 = 75.0% | 23 |
| codex | hybrid | precision-gap | 12 | 100% | 3346 | 7607 | 3871 | 14 | 2/12 = 16.7% | 38 |
| codex | no-graph | graph-strength | 12 | 100% | 13154 | 26230 | 14676 | 0 | 0/12 = 0.0% | 67 |
| codex | no-graph | payload-volume | 6 | 100% | 12434 | 24527 | 13374 | 0 | 0/6 = 0.0% | 36 |
| codex | no-graph | precision-gap | 12 | 100% | 11169 | 15969 | 8733 | 0 | 0/12 = 0.0% | 42 |
| codex | no-graph | selector-ambiguity | 6 | 100% | 19298 | 51473 | 21861 | 0 | 0/6 = 0.0% | 52 |

---

## Category Aggregate

`median go/ng` compares aggregate category medians. `mean fixture go/ng` and `worst fixture go/ng` are per-fixture ratios and are the load-bearing cost readings. Lower ratios are better for graph-only.

| provider | category | no-graph pass | graph-only pass | pass delta | median go/ng | mean fixture go/ng | worst fixture go/ng | hybrid graph rate |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| claude | graph-strength | 12/12 | 12/12 | +0 | 2.44x | 3.72x | 6.73x | 3/12 |
| claude | precision-gap | 12/12 | 12/12 | +0 | 5.41x | 4.65x | 7.61x | 0/12 |
| claude | payload-volume | 4/6 | 6/6 | +2 | 3.38x | 5.68x | 9.71x | n/a |
| claude | selector-ambiguity | 6/6 | 6/6 | +0 | 1.48x | 1.42x | 1.94x | n/a |
| codex (pre-fix) | graph-strength | 12/12 | 9/12 | -3 | 0.71x | 1.70x | 5.59x | 9/12 |
| codex (pre-fix) | precision-gap | 12/12 | 12/12 | +0 | 1.38x | 2.15x | 5.41x | 2/12 |
| codex (pre-fix) | payload-volume | 6/6 | 6/6 | +0 | 1.36x | 1.87x | 3.25x | n/a |
| codex (pre-fix) | selector-ambiguity | 6/6 | 3/6 | -3 | 1.14x | 1.45x | 1.89x | n/a |
| codex (post-fix) | graph-strength | 12/12 | 12/12 | +0 | 0.67x | 2.61x | 8.84x | 9/12 |
| codex (post-fix) | precision-gap | 12/12 | 12/12 | +0 | 0.79x | 1.53x | 2.99x | 2/12 |
| codex (post-fix) | payload-volume | 6/6 | 6/6 | +0 | 3.36x | 4.27x | 7.78x | n/a |
| codex (post-fix) | selector-ambiguity | 6/6 | 6/6 | +0 | 1.35x | 2.29x | 3.72x | n/a |

---

## Production vs Synthetic

| provider | mode | arm | runs | pass | median_total_tokens | graph_call_rate | graph_calls | failed_graph_calls |
|---|---|---|---:|---:|---:|---:|---:|---:|
| claude | production | no-graph | 21 | 19/21 | 1916 | 0/21 | 0 | 0 |
| claude | production | graph-only | 21 | 21/21 | 3720 | 21/21 | 357 | 6 |
| claude | production | hybrid | 9 | 9/9 | 1735 | 3/9 | 3 | 0 |
| claude | synthetic | no-graph | 15 | 15/15 | 219 | 0/15 | 0 | 0 |
| claude | synthetic | graph-only | 15 | 15/15 | 1283 | 15/15 | 79 | 3 |
| claude | synthetic | hybrid | 15 | 15/15 | 318 | 0/15 | 0 | 0 |
| codex | production | no-graph | 21 | 21/21 | 14162 | 0/21 | 0 | 0 |
| codex | production | graph-only (pre-fix) | 21 | 15/21 | 17847 | 21/21 | 222 | 18 |
| codex | production | graph-only (post-fix) | 21 | 21/21 | 24522 | 21/21 | 295 | 4 |
| codex | production | hybrid | 9 | 9/9 | 4872 | 3/9 | 3 | 0 |
| codex | synthetic | no-graph | 15 | 15/15 | 11278 | 0/15 | 0 | 0 |
| codex | synthetic | graph-only (pre-fix) | 15 | 15/15 | 14972 | 15/15 | 112 | 7 |
| codex | synthetic | graph-only (post-fix) | 15 | 15/15 | 8429 | 15/15 | 143 | 0 |
| codex | synthetic | hybrid | 15 | 15/15 | 3819 | 8/15 | 37 | 3 |

The production split is the load-bearing product signal. Pre-fix, Codex graph-only lost accuracy only on production-grounded fixtures. Post-fix, Codex graph-only passes all 36 cells but at higher production-side cost (24,522 median vs 14,162 no-graph) — the cost moved into the two formerly-failing fixtures, which now pass but at 8.84× and 3.72× their no-graph medians. Claude graph-only passed all production fixtures post-fix; Claude no-graph missed `const-value-extraction` twice.

---

## Codex Pre-Fix vs Post-Fix Comparison

The post-fix Codex graph-only rerun isolates the joint effect of `T20260425-0729` (string-list coercion) and `T20260425-0739` (pub-use re-export indexing). Identical fixture set, identical n=3 seeds-per-cell, identical provider/model (`gpt-5.3-codex`); only the harness SHA and graph index differ.

### Aggregate

| metric | pre-fix | post-fix | delta |
|---|---:|---:|---|
| graph-only pass rate | 30/36 | 36/36 | +6 |
| graph-only median tokens | 15,462 | 12,928 | -16% |
| graph-only p90 tokens | 64,877 | 71,774 | +11% |
| total graph calls | 334 | 438 | +31% |
| failed graph calls | 25 | 4 | -84% |
| schema-coercion failures | 26 | 0 | -100% |

The schema-coercion class (`refs.include must be array`, `pack.selectors must be array`) is fully resolved. Total graph calls went up because Codex now successfully completes calls that previously failed and forced retries; more useful tool calls produce more downstream calls. p90 went up because the two formerly-failing fixtures now pass at high cost rather than giving up early.

### Per-fixture flips and regressions

| fixture | class | pre-fix pass | post-fix pass | pre-fix median | post-fix median | post/pre ratio |
|---|---|---:|---:|---:|---:|---:|
| `reverse-export-orbit-error` | graph-strength | 0/3 | **3/3** | 77,792 | 122,948 | 1.58x |
| `module-surface-orbit-mcp` | selector-ambiguity | 0/3 | **3/3** | 11,134 | 21,885 | 1.97x |
| `const-value-extraction` | payload-volume | 3/3 | 3/3 | 28,222 | 67,515 | 2.39x |
| `generic-dispatch-concrete-impl` | precision-gap | 3/3 | 3/3 | 15,741 | 20,293 | 1.29x |
| `impl-divergence-trait-method` | payload-volume | 3/3 | 3/3 | 8,081 | 12,646 | 1.56x |
| `callers-2hop-graphbenchpolicy` | graph-strength | 3/3 | 3/3 | 5,444 | 8,532 | 1.57x |
| `deps-downstream-orbit-knowledge` | graph-strength | 3/3 | 3/3 | 2,250 | 2,833 | 1.26x |
| `implementors-benchsink-with-blanket` | graph-strength | 3/3 | 3/3 | 7,370 | 8,705 | 1.18x |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | 3/3 | 3/3 | 32,146 | 27,691 | 0.86x |
| `function-as-value-vs-direct-call` | precision-gap | 3/3 | 3/3 | 16,761 | 13,210 | 0.79x |
| `macro-expanded-callers` | precision-gap | 3/3 | 3/3 | 6,605 | 4,368 | 0.66x |
| `construct-vs-match-benchevent-distinct` | precision-gap | 3/3 | 3/3 | 15,257 | 8,429 | 0.55x |

The two formerly-failing fixtures pass at 1.58–1.97× their pre-fix cost — pre-fix Codex was bailing early into `[]` once graph confidently lied, so the pre-fix token count was an under-estimate of "what it actually takes to answer this question with the graph." Post-fix, Codex does the real work and we see the real cost.

Four fixtures got cheaper post-fix (`function-as-value`, `macro-expanded`, `construct-vs-match`, `references-vs-callers`); these are cells where the schema-coercion friction was the dominant pre-fix overhead. Six fixtures got more expensive — most by a small amount. `const-value-extraction` is the largest "passed-then-passed-more-expensively" gap (2.39×) and probably reflects a richer post-fix index returning more candidates that Codex enumerates through.

---

## Claude Per-Fixture Table

| fixture | class | mode | arm | pass | median_tokens | p90_tokens | graph_call_rate | graph_calls | failed_graph_calls | shell/fs_calls |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | no-graph | 3/3 | 535 | 746 | 0/3 | 0 | 0 | 3 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | graph-only | 3/3 | 1016 | 1283 | 3/3 | 6 | 0 | 0 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | hybrid | 3/3 | 750 | 855 | 0/3 | 0 | 0 | 3 |
| `const-value-extraction` | payload-volume | production | no-graph | 1/3 | 719 | 1198 | 0/3 | 0 | 0 | 14 |
| `const-value-extraction` | payload-volume | production | graph-only | 3/3 | 6979 | 11584 | 3/3 | 158 | 0 | 0 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | no-graph | 3/3 | 481 | 699 | 0/3 | 0 | 0 | 3 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | graph-only | 3/3 | 2290 | 2370 | 3/3 | 30 | 0 | 0 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | hybrid | 3/3 | 719 | 732 | 0/3 | 0 | 0 | 3 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | no-graph | 3/3 | 1610 | 1940 | 0/3 | 0 | 0 | 19 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | graph-only | 3/3 | 1338 | 1959 | 3/3 | 3 | 0 | 0 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | hybrid | 3/3 | 1735 | 1888 | 3/3 | 3 | 0 | 0 |
| `function-as-value-vs-direct-call` | precision-gap | production | no-graph | 3/3 | 2308 | 3371 | 0/3 | 0 | 0 | 21 |
| `function-as-value-vs-direct-call` | precision-gap | production | graph-only | 3/3 | 4273 | 4652 | 3/3 | 24 | 0 | 0 |
| `function-as-value-vs-direct-call` | precision-gap | production | hybrid | 3/3 | 2823 | 2915 | 0/3 | 0 | 0 | 24 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | no-graph | 3/3 | 197 | 219 | 0/3 | 0 | 0 | 3 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | graph-only | 3/3 | 864 | 1196 | 3/3 | 13 | 0 | 0 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | hybrid | 3/3 | 219 | 239 | 0/3 | 0 | 0 | 3 |
| `impl-divergence-trait-method` | payload-volume | production | no-graph | 3/3 | 2004 | 2508 | 0/3 | 0 | 0 | 20 |
| `impl-divergence-trait-method` | payload-volume | production | graph-only | 3/3 | 3332 | 3438 | 3/3 | 43 | 0 | 0 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | no-graph | 3/3 | 184 | 184 | 0/3 | 0 | 0 | 3 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | graph-only | 3/3 | 998 | 1537 | 3/3 | 7 | 0 | 0 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | hybrid | 3/3 | 318 | 327 | 0/3 | 0 | 0 | 3 |
| `macro-expanded-callers` | precision-gap | synthetic | no-graph | 3/3 | 203 | 231 | 0/3 | 0 | 0 | 3 |
| `macro-expanded-callers` | precision-gap | synthetic | graph-only | 3/3 | 1545 | 1558 | 3/3 | 23 | 3 | 0 |
| `macro-expanded-callers` | precision-gap | synthetic | hybrid | 3/3 | 203 | 219 | 0/3 | 0 | 0 | 3 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | no-graph | 3/3 | 1916 | 2285 | 0/3 | 0 | 0 | 10 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | graph-only | 3/3 | 3720 | 4383 | 3/3 | 28 | 1 | 0 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | no-graph | 3/3 | 4698 | 5296 | 0/3 | 0 | 0 | 49 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | graph-only | 3/3 | 4201 | 7645 | 3/3 | 40 | 1 | 0 |
| `reverse-export-orbit-error` | graph-strength | production | no-graph | 3/3 | 537 | 707 | 0/3 | 0 | 0 | 6 |
| `reverse-export-orbit-error` | graph-strength | production | graph-only | 3/3 | 3613 | 4086 | 3/3 | 61 | 4 | 0 |
| `reverse-export-orbit-error` | graph-strength | production | hybrid | 3/3 | 588 | 629 | 0/3 | 0 | 0 | 6 |

## Codex Per-Fixture Table

The `graph-only (pre-fix)` rows are retained from the original Codex sweep. The `graph-only (post-fix)` rows are from the rerun at harness SHA `56a9c07b...` after both `T20260425-0729` and `T20260425-0739` landed.

| fixture | class | mode | arm | pass | median_tokens | p90_tokens | graph_call_rate | graph_calls | failed_graph_calls | shell/fs_calls |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | no-graph | 3/3 | 12397 | 23855 | 0/3 | 0 | 0 | 10 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | graph-only (pre-fix) | 3/3 | 5444 | 15280 | 3/3 | 16 | 1 | 0 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | graph-only (post-fix) | 3/3 | 8532 | 8858 | 3/3 | 28 | 0 | 0 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | hybrid | 3/3 | 5495 | 5891 | 3/3 | 14 | 0 | 1 |
| `const-value-extraction` | payload-volume | production | no-graph | 3/3 | 8680 | 9445 | 0/3 | 0 | 0 | 21 |
| `const-value-extraction` | payload-volume | production | graph-only (pre-fix) | 3/3 | 28222 | 101587 | 3/3 | 33 | 2 | 0 |
| `const-value-extraction` | payload-volume | production | graph-only (post-fix) | 3/3 | 67515 | 74329 | 3/3 | 61 | 2 | 0 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | no-graph | 3/3 | 2818 | 12423 | 0/3 | 0 | 0 | 8 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | graph-only (pre-fix) | 3/3 | 15257 | 17551 | 3/3 | 20 | 1 | 0 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | graph-only (post-fix) | 3/3 | 8429 | 15958 | 3/3 | 22 | 0 | 0 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | hybrid | 3/3 | 2873 | 4957 | 0/3 | 0 | 0 | 6 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | no-graph | 3/3 | 16379 | 25955 | 0/3 | 0 | 0 | 30 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | graph-only (pre-fix) | 3/3 | 2250 | 11335 | 3/3 | 6 | 0 | 0 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | graph-only (post-fix) | 3/3 | 2833 | 11605 | 3/3 | 6 | 0 | 0 |
| `deps-downstream-orbit-knowledge` | graph-strength | production | hybrid | 3/3 | 1383 | 1449 | 3/3 | 3 | 0 | 0 |
| `function-as-value-vs-direct-call` | precision-gap | production | no-graph | 3/3 | 14162 | 16744 | 0/3 | 0 | 0 | 23 |
| `function-as-value-vs-direct-call` | precision-gap | production | graph-only (pre-fix) | 3/3 | 16761 | 23051 | 3/3 | 27 | 3 | 0 |
| `function-as-value-vs-direct-call` | precision-gap | production | graph-only (post-fix) | 3/3 | 13210 | 17944 | 3/3 | 26 | 0 | 0 |
| `function-as-value-vs-direct-call` | precision-gap | production | hybrid | 3/3 | 4874 | 8743 | 0/3 | 0 | 0 | 21 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | no-graph | 3/3 | 11132 | 11207 | 0/3 | 0 | 0 | 4 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | graph-only (pre-fix) | 3/3 | 15741 | 24846 | 3/3 | 22 | 1 | 0 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | graph-only (post-fix) | 3/3 | 20293 | 37761 | 3/3 | 39 | 0 | 0 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | hybrid | 3/3 | 3819 | 4741 | 2/3 | 14 | 1 | 4 |
| `impl-divergence-trait-method` | payload-volume | production | no-graph | 3/3 | 16776 | 24527 | 0/3 | 0 | 0 | 15 |
| `impl-divergence-trait-method` | payload-volume | production | graph-only (pre-fix) | 3/3 | 8081 | 17847 | 3/3 | 24 | 1 | 0 |
| `impl-divergence-trait-method` | payload-volume | production | graph-only (post-fix) | 3/3 | 12646 | 22257 | 3/3 | 45 | 0 | 0 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | no-graph | 3/3 | 11469 | 22553 | 0/3 | 0 | 0 | 7 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | graph-only (pre-fix) | 3/3 | 7370 | 34916 | 3/3 | 32 | 2 | 0 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | graph-only (post-fix) | 3/3 | 8705 | 10149 | 3/3 | 34 | 0 | 0 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | hybrid | 3/3 | 3981 | 14873 | 3/3 | 9 | 2 | 7 |
| `macro-expanded-callers` | precision-gap | synthetic | no-graph | 3/3 | 11278 | 11512 | 0/3 | 0 | 0 | 7 |
| `macro-expanded-callers` | precision-gap | synthetic | graph-only (pre-fix) | 3/3 | 6605 | 14098 | 3/3 | 22 | 2 | 0 |
| `macro-expanded-callers` | precision-gap | synthetic | graph-only (post-fix) | 3/3 | 4368 | 4606 | 3/3 | 20 | 0 | 0 |
| `macro-expanded-callers` | precision-gap | synthetic | hybrid | 3/3 | 2288 | 2485 | 0/3 | 0 | 0 | 7 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | no-graph | 3/3 | 5886 | 7437 | 0/3 | 0 | 0 | 16 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | graph-only (pre-fix) | 0/3 | 11134 | 19582 | 3/3 | 42 | 5 | 0 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | graph-only (post-fix) | 3/3 | 21885 | 26781 | 3/3 | 41 | 0 | 0 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | no-graph | 3/3 | 32141 | 51473 | 0/3 | 0 | 0 | 36 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | graph-only (pre-fix) | 3/3 | 32146 | 37822 | 3/3 | 29 | 4 | 0 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | graph-only (post-fix) | 3/3 | 27691 | 31259 | 3/3 | 31 | 1 | 0 |
| `reverse-export-orbit-error` | graph-strength | production | no-graph | 3/3 | 13912 | 26349 | 0/3 | 0 | 0 | 20 |
| `reverse-export-orbit-error` | graph-strength | production | graph-only (pre-fix) | 0/3 | 77792 | 78697 | 3/3 | 61 | 3 | 0 |
| `reverse-export-orbit-error` | graph-strength | production | graph-only (post-fix) | 3/3 | 122948 | 141342 | 3/3 | 85 | 1 | 0 |
| `reverse-export-orbit-error` | graph-strength | production | hybrid | 3/3 | 5830 | 13354 | 0/3 | 0 | 0 | 15 |

---

## Claude Hybrid Utilization

| fixture | pass | median_tokens | graph_call_rate | graph_calls | shell/fs_calls | interpretation |
|---|---:|---:|---:|---:|---:|---|
| `callers-2hop-graphbenchpolicy` | 3/3 | 750 | 0/3 | 0 | 3 | Passed by shell/source fallback; graph avoided organically. |
| `construct-vs-match-benchevent-distinct` | 3/3 | 719 | 0/3 | 0 | 3 | Passed by shell/source fallback; graph avoided organically. |
| `deps-downstream-orbit-knowledge` | 3/3 | 1735 | 3/3 | 3 | 0 | Only Claude hybrid fixture that used graph; direct deps solved it cleanly. |
| `function-as-value-vs-direct-call` | 3/3 | 2823 | 0/3 | 0 | 24 | Passed by source fallback with relatively heavy shell/read use. |
| `generic-dispatch-concrete-impl` | 3/3 | 219 | 0/3 | 0 | 3 | Passed by direct source inspection; graph avoided organically. |
| `implementors-benchsink-with-blanket` | 3/3 | 318 | 0/3 | 0 | 3 | Passed by direct source inspection; graph avoided organically. |
| `macro-expanded-callers` | 3/3 | 203 | 0/3 | 0 | 3 | Passed by direct source inspection; graph avoided organically. |
| `reverse-export-orbit-error` | 3/3 | 588 | 0/3 | 0 | 6 | Passed by shell/source fallback despite graph-only success post-fix. |

Claude hybrid shows that neutral hybrid prompting does not guarantee graph selection. It mostly measures whether Claude can route to the cheapest available source strategy, and for this fixture set that was usually shell/source reading rather than graph.

## Codex Hybrid Utilization

| fixture | pass | median_tokens | graph_call_rate | graph_calls | shell/fs_calls | interpretation |
|---|---:|---:|---:|---:|---:|---|
| `callers-2hop-graphbenchpolicy` | 3/3 | 5495 | 3/3 | 14 | 1 | Used graph consistently; stayed well below no-graph. |
| `construct-vs-match-benchevent-distinct` | 3/3 | 2873 | 0/3 | 0 | 6 | Passed by shell fallback; graph avoided organically. |
| `deps-downstream-orbit-knowledge` | 3/3 | 1383 | 3/3 | 3 | 0 | Best graph-shaped win; deps solved directly. |
| `function-as-value-vs-direct-call` | 3/3 | 4874 | 0/3 | 0 | 21 | Passed by shell fallback; graph avoided organically. |
| `generic-dispatch-concrete-impl` | 3/3 | 3819 | 2/3 | 14 | 4 | Mixed graph use; source reading did the final disambiguation. |
| `implementors-benchsink-with-blanket` | 3/3 | 3981 | 3/3 | 9 | 7 | Used graph, then shell/source checks; cheaper than both baselines. |
| `macro-expanded-callers` | 3/3 | 2288 | 0/3 | 0 | 7 | Passed by shell fallback; graph avoided organically. |
| `reverse-export-orbit-error` | 3/3 | 5830 | 0/3 | 0 | 15 | Passed by shell fallback; graph-only failed all seeds. |

Codex hybrid's 24/24 pass rate is not proof that every hybrid-eligible fixture is graph-shaped. It is proof that Codex can route around graph gaps when shell/source tools are available.

---

## Claude Graph-Only Cost Ratios

| fixture | class | mode | no-graph median | graph-only median | go/ng | graph-only pass |
|---|---|---|---:|---:|---:|---:|
| `deps-downstream-orbit-knowledge` | graph-strength | production | 1610 | 1338 | 0.83x | 3/3 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | 4698 | 4201 | 0.89x | 3/3 |
| `impl-divergence-trait-method` | payload-volume | production | 2004 | 3332 | 1.66x | 3/3 |
| `function-as-value-vs-direct-call` | precision-gap | production | 2308 | 4273 | 1.85x | 3/3 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | 535 | 1016 | 1.90x | 3/3 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | 1916 | 3720 | 1.94x | 3/3 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | 197 | 864 | 4.39x | 3/3 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | 481 | 2290 | 4.76x | 3/3 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | 184 | 998 | 5.42x | 3/3 |
| `reverse-export-orbit-error` | graph-strength | production | 537 | 3613 | 6.73x | 3/3 |
| `macro-expanded-callers` | precision-gap | synthetic | 203 | 1545 | 7.61x | 3/3 |
| `const-value-extraction` | payload-volume | production | 719 | 6979 | 9.71x | 3/3 |

Claude graph-only was excellent for accuracy after the graph fixes, but it was rarely the cheapest route. The `const-value-extraction` cell is the sharpest tradeoff: graph-only found the full set in every seed, while no-graph missed one constant twice, but graph-only used 9.71x the no-graph median tokens.

## Codex Graph-Only Cost Ratios (pre-fix)

| fixture | class | mode | no-graph median | graph-only median | go/ng | graph-only pass |
|---|---|---|---:|---:|---:|---:|
| `deps-downstream-orbit-knowledge` | graph-strength | production | 16379 | 2250 | 0.14x | 3/3 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | 12397 | 5444 | 0.44x | 3/3 |
| `impl-divergence-trait-method` | payload-volume | production | 16776 | 8081 | 0.48x | 3/3 |
| `macro-expanded-callers` | precision-gap | synthetic | 11278 | 6605 | 0.59x | 3/3 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | 11469 | 7370 | 0.64x | 3/3 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | 32141 | 32146 | 1.00x | 3/3 |
| `function-as-value-vs-direct-call` | precision-gap | production | 14162 | 16761 | 1.18x | 3/3 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | 11132 | 15741 | 1.41x | 3/3 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | 5886 | 11134 | 1.89x | 0/3 |
| `const-value-extraction` | payload-volume | production | 8680 | 28222 | 3.25x | 3/3 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | 2818 | 15257 | 5.41x | 3/3 |
| `reverse-export-orbit-error` | graph-strength | production | 13912 | 77792 | 5.59x | 0/3 |

## Codex Graph-Only Cost Ratios (post-fix)

| fixture | class | mode | no-graph median | graph-only median | go/ng | graph-only pass |
|---|---|---|---:|---:|---:|---:|
| `deps-downstream-orbit-knowledge` | graph-strength | production | 16379 | 2833 | 0.17x | 3/3 |
| `macro-expanded-callers` | precision-gap | synthetic | 11278 | 4368 | 0.39x | 3/3 |
| `construct-vs-match-benchevent-distinct` | precision-gap | synthetic | 2818 | 8429 | 2.99x | 3/3 |
| `callers-2hop-graphbenchpolicy` | graph-strength | synthetic | 12397 | 8532 | 0.69x | 3/3 |
| `impl-divergence-trait-method` | payload-volume | production | 16776 | 12646 | 0.75x | 3/3 |
| `implementors-benchsink-with-blanket` | graph-strength | synthetic | 11469 | 8705 | 0.76x | 3/3 |
| `references-vs-callers-tool-registry-register` | selector-ambiguity | production | 32141 | 27691 | 0.86x | 3/3 |
| `function-as-value-vs-direct-call` | precision-gap | production | 14162 | 13210 | 0.93x | 3/3 |
| `generic-dispatch-concrete-impl` | precision-gap | synthetic | 11132 | 20293 | 1.82x | 3/3 |
| `module-surface-orbit-mcp` | selector-ambiguity | production | 5886 | 21885 | 3.72x | **3/3** |
| `const-value-extraction` | payload-volume | production | 8680 | 67515 | 7.78x | 3/3 |
| `reverse-export-orbit-error` | graph-strength | production | 13912 | 122948 | 8.84x | **3/3** |

Post-fix Codex graph-only beats no-graph on 7/12 fixtures (vs 5/12 pre-fix). The losses are concentrated in two fixtures where pre-fix Codex was bailing early into wrong answers (`reverse-export`, `module-surface`); post-fix it does the real work and the cost surfaces. The third large outlier is `const-value-extraction` (7.78x) — a payload-volume fixture where graph enumeration is structurally complete but expensive on a value-extraction task.

---

## Claude Tool Diagnostics

Per-tool response size is measured in response characters, not model tokens. The transcripts do not expose per-tool token attribution.

| tool | invocations | succeeded | failed | success_rate | median_response_chars | p90_response_chars |
|---|---:|---:|---:|---:|---:|---:|
| callers | 7 | 4 | 3 | 57% | 1838 | 29660 |
| refs | 14 | 10 | 4 | 71% | 3945 | 18990 |
| implementors | 6 | 6 | 0 | 100% | 1122 | 1249 |
| deps | 6 | 6 | 0 | 100% | 606 | 606 |
| pack | 2 | 1 | 1 | 50% | 979 | 979 |
| search | 80 | 80 | 0 | 100% | 212 | 1427 |
| show | 324 | 323 | 1 | 100% | 762 | 2931 |
| overview | 0 | 0 | 0 | n/a | - | - |

Failed graph calls by message:

| message | count | affected tools |
|---|---:|---|
| `invalid input: include entries must be code, doc, config, or all, got ["code"]` | 3 | `refs` |
| `execution failed: selector BenchDerivedStruct::default:fn does not resolve to a node` | 2 | `callers` |
| `execution failed: selector BenchDerivedStruct::default does not resolve to a node` | 1 | `callers` |
| `invalid input: invalid selector ["file:crates/orbit-mcp/src/lib.rs"]` | 1 | `pack` |
| `invalid input: selector file:crates/orbit-common/src/types.rs does not resolve to a node` | 1 | `show` |
| `invalid input: include entries must be code, doc, config, or all, got ["code", "config"]` | 1 | `refs` |

These failures are not the same shape as the pre-fix Codex scalar-list failures. The remaining Claude failures are nested-list/invalid-selector mistakes and a derive/default selector expectation that graph does not support.

## Codex Tool Diagnostics (pre-fix)

Per-tool response size is measured in response characters, not model tokens. The transcripts do not expose per-tool token attribution.

| tool | invocations | succeeded | failed | success_rate | median_response_chars | p90_response_chars |
|---|---:|---:|---:|---:|---:|---:|
| callers | 16 | 16 | 0 | 100% | 1838 | 382947 |
| refs | 61 | 39 | 22 | 64% | 494 | 90616 |
| implementors | 13 | 12 | 1 | 92% | 1122 | 1249 |
| deps | 6 | 6 | 0 | 100% | 606 | 606 |
| pack | 69 | 65 | 4 | 94% | 1724 | 19595 |
| search | 103 | 102 | 1 | 99% | 260 | 2653 |
| show | 84 | 84 | 0 | 100% | 777 | 1587 |
| overview | 22 | 22 | 0 | 100% | 2436 | 59717 |

Failed graph calls by message:

| message | count | affected tools |
|---|---:|---|
| `invalid input: include must be an array of strings` | 22 | `refs` |
| `invalid input: selectors must be an array` | 4 | `pack` |
| `invalid input: query must not be empty` | 1 | `search` |
| `invalid input: invalid selector BenchAuditSink` | 1 | `implementors` |

The `refs.include` and `pack.selectors` scalar-list failures were addressed by `T20260425-0729` before the post-fix rerun.

## Codex Tool Diagnostics (post-fix)

| tool | invocations | succeeded | failed | success_rate | median_response_chars | p90_response_chars |
|---|---:|---:|---:|---:|---:|---:|
| show | 131 | 131 | 0 | 100% | 2181 | 4018 |
| pack | 102 | 102 | 0 | 100% | 6261 | 53902 |
| search | 101 | 99 | 2 | 98% | 617 | 14112 |
| refs | 45 | 43 | 2 | 95% | 1941 | 116359 |
| overview | 24 | 24 | 0 | 100% | 3626 | 128204 |
| callers | 23 | 23 | 0 | 100% | 3967 | 319274 |
| implementors | 9 | 9 | 0 | 100% | 2181 | 2689 |
| deps | 3 | 3 | 0 | 100% | 1396 | 1396 |

Failed graph calls by message:

| message | count | affected tools |
|---|---:|---|
| `invalid input: query must not be empty` | 2 | `search` |
| `invalid input: invalid selector ToolRegistry::register: selectors must start with dir:, file:, or symbol:` | 1 | `refs` |
| `invalid input: invalid selector OrbitError: selectors must start with dir:, file:, or symbol:` | 1 | `refs` |

The schema-coercion class (scalar-as-array) is fully resolved — 26 of the pre-fix 28 failures are gone. Remaining failures are query/selector ergonomics: empty query strings, and missing `symbol:`/`file:`/`dir:` prefixes on selectors. Both classes are recoverable by the agent on retry, but the latter is the same shape Claude hits post-fix and is a candidate for the next ergonomics task.

Tool-mix shift is also notable: post-fix Codex now leans on `show` (131 invocations vs 84 pre-fix) and `pack` (102 vs 69) — direct file/symbol inspection — instead of `refs` (45 vs 61) where the schema-coercion friction lived.

---

## Failure Taxonomy

Non-passing runs:

| provider | arm | fixture | seeds | classification | observed answer |
|---|---|---|---|---|---|
| claude | no-graph | `const-value-extraction` | 1, 2 | source-search miss | omitted `V2_TOOL_WILDCARD_ROOTS`; seed 3 found the full set |
| codex | graph-only (pre-fix) | `module-surface-orbit-mcp` | 1, 2, 3 | known graph bug / root-surface gap (`T20260425-0739`) | returned `McpHost`, `serve_stdio`; excluded `OrbitToolServer`. **Resolved post-fix: 3/3 pass.** |
| codex | graph-only (pre-fix) | `reverse-export-orbit-error` | 1, 2, 3 | known graph bug / re-export metadata gap (`T20260425-0739`) | returned `[]`; excluded the original definition. **Resolved post-fix: 3/3 pass.** |

Anomaly flags are not mutually exclusive. `Primary` means the row count emitted by `aggregate.py`'s precedence-ordered taxonomy; `independent` means the flag was true even if another flag won precedence.

| provider | flag | runs | notes |
|---|---|---:|---|
| claude | schema-coercion | 8 primary | 9 failed graph calls total; all recovered. Remaining shapes are nested-list/invalid-selector errors, not the pre-fix scalar-list issue. |
| claude | payload-firehose | 7 primary / 13 independent | Concentrated in graph-only `const-value-extraction`, `macro-expanded-callers`, `reverse-export-orbit-error`, `implementors-benchsink-with-blanket`, and one `generic-dispatch-concrete-impl` seed. |
| claude | wrong-tool | 0 | No graph-only Claude run failed. |
| claude | design-defect | 21 | Hybrid passed with zero graph calls. Interpret as "organic selection avoided graph", not as a correctness failure. |
| codex (pre-fix) | schema-coercion | 25 primary | 28 failed graph calls total; most were recovered by retrying with array-shaped args. |
| codex (pre-fix) | payload-firehose | 2 primary / 6 independent | Primary taxonomy hides several firehose runs behind schema-coercion. |
| codex (pre-fix) | wrong-tool | 6 | The six pre-fix graph-only non-passing runs above. |
| codex (pre-fix) | design-defect | 13 | Hybrid passed with zero graph calls. Interpret as "organic selection avoided graph", not as a correctness failure. |
| codex (post-fix) | schema-coercion | 4 primary | All 4 are query/selector ergonomics (empty query, missing `dir:`/`file:`/`symbol:` prefix), not the pre-fix scalar-list class. |
| codex (post-fix) | payload-firehose | 4 primary | Concentrated in `const-value-extraction` and `reverse-export-orbit-error` — fixtures where graph enumerates many candidates and Codex pages through them. |
| codex (post-fix) | wrong-tool | 0 | All 36 graph-only cells passed. |
| codex (post-fix) | design-defect | 13 | Same hybrid runs as pre-fix; hybrid was not rerun. |

---

## Standout Fixtures

Top graph-only wins by token reduction:

| provider | fixture | result |
|---|---|---|
| codex (pre-fix) | `deps-downstream-orbit-knowledge` | 3/3 pass, 0.14x no-graph tokens |
| codex (post-fix) | `deps-downstream-orbit-knowledge` | 3/3 pass, 0.17x no-graph tokens |
| codex (post-fix) | `macro-expanded-callers` | 3/3 pass, 0.39x no-graph tokens |
| codex (pre-fix) | `callers-2hop-graphbenchpolicy` | 3/3 pass, 0.44x no-graph tokens |
| codex (pre-fix) | `impl-divergence-trait-method` | 3/3 pass, 0.48x no-graph tokens |
| claude | `deps-downstream-orbit-knowledge` | 3/3 pass, 0.83x no-graph tokens |
| claude | `references-vs-callers-tool-registry-register` | 3/3 pass, 0.89x no-graph tokens |

Accuracy standouts:

| provider | fixture | result |
|---|---|---|
| claude + codex (post-fix) | `reverse-export-orbit-error` | both providers graph-only 3/3 post-fix; Codex pre-fix was 0/3 |
| claude + codex (post-fix) | `module-surface-orbit-mcp` | both providers graph-only 3/3 post-fix; Codex pre-fix was 0/3 |
| claude | `const-value-extraction` | graph-only 3/3 while no-graph was 1/3 |

Top graph-only losses:

| provider | fixture | result |
|---|---|---|
| codex (post-fix) | `reverse-export-orbit-error` | 3/3 pass, 8.84x no-graph tokens (pre-fix was 0/3 at 5.59x) |
| codex (post-fix) | `const-value-extraction` | 3/3 pass, 7.78x no-graph tokens |
| claude | `const-value-extraction` | 3/3 pass, 9.71x no-graph tokens |
| claude | `macro-expanded-callers` | 3/3 pass, 7.61x no-graph tokens |
| claude | `reverse-export-orbit-error` | 3/3 pass post-fix, 6.73x no-graph tokens |
| codex (post-fix) | `module-surface-orbit-mcp` | 3/3 pass, 3.72x no-graph tokens (pre-fix was 0/3 at 1.89x) |

---

## Interpretation

The full v4 result — now with both providers post-fix on graph-only — supports keeping graph as an optional navigation surface, not as a replacement for source reads. Graph is excellent when the question maps directly to a precise graph primitive, with `deps-downstream-orbit-knowledge` the cleanest repeated win across both providers (0.17x for Codex post-fix, 0.83x for Claude).

`T20260425-0739` was the right diagnosis. The post-fix Codex rerun confirms it directly: both `reverse-export-orbit-error` and `module-surface-orbit-mcp` flipped from 0/3 to 3/3 with no other changes. With Codex now post-fix, the Codex-vs-Claude comparison is clean — and the residual cost difference (Codex graph-only median 12,928 vs Claude graph-only median 2,330) is the load-bearing provider-behavior signal, not a tool-bug artifact.

The post-fix data also exposes a subtler pattern: **fixing the bug increased cost on the formerly-failing fixtures.** Pre-fix Codex bailed early on `reverse-export-orbit-error` (~78k tokens to fail); post-fix it does the real work and pays ~123k tokens to succeed. The pre-fix cost ratios on those two fixtures were under-estimates of "what graph-only actually costs to answer this question." Post-fix cost ratios are the honest reading.

Hybrid is still the practical success case, but its meaning differs by provider. Codex selectively used graph and got the strongest overall cost/correctness profile on the hybrid subset. Claude mostly avoided graph in hybrid, so its 24/24 result is better read as "source fallback remains essential" than "graph was selected well." The post-fix rerun does not change this — Codex hybrid was not rerun, but its 24/24 + 11/24 graph-call rate is unchallenged.

The highest-leverage next steps are:

1. Add payload shaping for high-cardinality responses (`refs`, `overview`, `callers`, and repeated `show`) so graph-only cannot spend 6x-10x tokens on enumeration. The post-fix `reverse-export` (8.84x) and `const-value-extraction` (7.78x) cells are the load-bearing examples.
2. Tighten selector ergonomics: post-fix Codex still hit 4 failed graph calls — 2 empty queries and 2 unprefixed selectors (`OrbitError`, `ToolRegistry::register` instead of `symbol:OrbitError`). Claude hits the same shape post-fix. Both providers want a forgiving selector parser.
3. Add a small hybrid-selection round with explicit "prefer graph when it directly answers the relationship; fall back to source for bodies/values" guidance. Neutral hybrid prompts measure organic tool choice, and Claude's organic choice was mostly "do not use graph."
4. Optionally rerun Codex hybrid post-fix to close out tool-fix confound entirely. Expected delta: 3 failed graph calls → 0; pass rate stays at 24/24; median tokens drop slightly. Skipped here on cost grounds.
5. Keep fixture-level ratios as the main cost metric. Aggregate medians hide both the `deps` win and the expensive-but-correct post-fix `reverse-export` result.

---

## Reproduction

Aggregate tables:

```bash
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/aggregate.py \
  --runs benchmarks/graph/v4/runs \
  --tasks benchmarks/graph/v4/tasks
```

Completed Codex sweeps:

The original `no-graph` and `graph-only` Codex artifacts are pre-fix for `T20260425-0729` and `T20260425-0739`. Codex `graph-only` was rerun post-fix on 2026-04-25; the pre-fix `graph-only` artifacts now live at `_archive/codex-graph-only-pre-fix-T20260425-0739/`.

```bash
# Pre-fix Codex no-graph + graph-only (graph-only artifacts later archived)
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/sweep.py \
  --provider codex --arms no-graph graph-only --n 3
```

```bash
# Pre-fix Codex hybrid (not rerun post-fix; passed 24/24)
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/sweep.py \
  --provider codex --arms hybrid --n 3 \
  --tasks callers-2hop-graphbenchpolicy construct-vs-match-benchevent-distinct \
  deps-downstream-orbit-knowledge function-as-value-vs-direct-call \
  generic-dispatch-concrete-impl implementors-benchsink-with-blanket \
  macro-expanded-callers reverse-export-orbit-error
```

```bash
# Post-fix Codex graph-only rerun (after archiving pre-fix dir)
mv benchmarks/graph/v4/runs/codex/graph-only \
   benchmarks/graph/v4/_archive/codex-graph-only-pre-fix-T20260425-0739

GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/sweep.py \
  --provider codex --arms graph-only --n 3
```

To re-aggregate the pre-fix data, temporarily symlink the archived dir back into `runs/codex/graph-only` (or pass a different `--runs` root pointing at `_archive/`).

Completed Claude sweeps:

```bash
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/sweep.py \
  --provider claude --arms no-graph graph-only --n 3
```

```bash
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/sweep.py \
  --provider claude --arms hybrid --n 3 \
  --tasks callers-2hop-graphbenchpolicy construct-vs-match-benchevent-distinct \
  deps-downstream-orbit-knowledge function-as-value-vs-direct-call \
  generic-dispatch-concrete-impl implementors-benchsink-with-blanket \
  macro-expanded-callers reverse-export-orbit-error
```
