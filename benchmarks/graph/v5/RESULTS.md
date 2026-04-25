# Graph Token-Usage Benchmark — v5 (Feature-Validation Round)

**Status:** Closed — final round of the v1-v5 series.
**Scope:** Focused validation of `source_regex` (the [T20260425-2140] tool change), not a full sweep. Codex only, `graph-only` arm only, 3 fixtures × 3 seeds = 9 cells.
**Sweep ID / seed:** `20260425-115117-21a938` / `142643867`
**Harness SHA:** `56a9c07b...` plus the `T20260425-2140` source_regex implementation.
**Why the small scope:** v4 motivated a specific tool-surface change (`source_regex`). v5 measures whether that change delivered. A full 192-cell sweep would have re-measured shapes already characterized in v4 without changing the conclusion. See [reading guide](../RESULTS.md#reading-guide) in the series wrap-up.

---

## Headline

**Feature works, agents over-use it.** All 9 cells passed, and where the regex is a natural fit the call-count and token wins are large. But on two of three fixtures, agents iterated `source_regex` 7-19 times per seed (typically with the same regex but narrower prefixes) instead of the predicted 1-2 calls. Tool surface is correct; the affordance is what missed.

---

## Pre-Fix vs Post-Fix Comparison

Baseline = post-T20260425-0739 Codex graph-only data from v4 (preserved at `v4/_archive/codex-graph-only-pre-T20260425-2140/`). Post-fix = this round.

| fixture | arm | pass | median tokens | median calls/seed | source_regex calls/seed (post) |
|---|---|---|---:|---:|---:|
| `const-value-extraction` | baseline | 3/3 | 67,515 | 23 | n/a |
| `const-value-extraction` | post-fix | 3/3 | **27,175 (-60%)** | **5 (-78%)** | 2, 3, 5 |
| `reverse-export-orbit-error` | baseline | 3/3 | 122,948 | 28 | n/a |
| `reverse-export-orbit-error` | post-fix | 3/3 | **45,907 (-63%)** | 22 (-21%) | **15, 19, 7** |
| `module-surface-orbit-mcp` | baseline | 3/3 | 21,885 | 17 | n/a |
| `module-surface-orbit-mcp` | post-fix | 3/3 | 24,533 (+12%) | 16 (-6%) | 4, 7, 8 |

---

## Per-Fixture Diagnosis

**`const-value-extraction` — clean win.** The "find every `pub const` in this module" question maps directly onto `source_regex: "^\s*pub\s+const\s+"` plus a broad prefix. Agents wrote essentially that, ran it once or twice, and stopped. Acceptance criterion (`≤6 calls/seed`) met cleanly.

**`reverse-export-orbit-error` — half-worked.** Tokens fell hard (-63%) because each `source_regex` call returns a small answer set instead of the 6,300-char `pack` payloads it replaced. But agents called the same regex (`pub\s+use\s+.*OrbitError`) 7-19 times per seed, varying the prefix per crate. The tool can answer the question in one call; agents iterated. AC ceiling (≤5 calls/seed) missed. Failure mode is agent affordance, not feature design.

**`module-surface-orbit-mcp` — mismatched shape.** The fixture asks "what's exported from `orbit-mcp`" — a structural question the regex form doesn't compress well. Agents tried regexes like `pub use|pub trait|pub fn` and then had to filter false positives manually. Net token cost rose ~12%. This is the residual case the original kind-taxonomy proposal would have covered; regex isn't the right tool here.

---

## What Changed in Response

The agent over-iteration on `reverse-export` is the load-bearing finding. Two-layer remediation:

1. **`orbit-graph` skill update** ([commit `1d306f03`](https://github.com/danieljhkim/orbit)) — added a "Source-Regex Enumeration" section with the explicit "ONE broad prefix + ONE regex, do NOT iterate" rule, three worked example shapes, and a verification-loop anti-pattern callout in the Stop Rule. Strategic guidance now lives in the skill, where it belongs, not stuffed into the MCP tool description.
2. **MCP tool descriptions remained terse** — the parameter contracts only. Strategy lives in the skill; tool descriptions describe the tool. (See series wrap-up §4 for the broader principle.)

The skill ships to both Claude (via `.claude/skills/orbit-graph`) and Codex (via `.agents/skills/orbit-graph`) from a single source-of-truth at `crates/orbit-core/assets/skills/orbit-graph/SKILL.md`. Cross-provider parity is automatic.

A re-run of these 3 fixtures after the skill update would test the affordance fix. Deferred — see the series wrap-up for why we stopped here.

---

## Failure Taxonomy

No `wrong-tool` runs (all 9 passed). 4 `schema-coercion` failures across the round, all from `source_regex` calls that hit the bounded-scan rule (`source_regex` without `prefix` or non-empty `query` and large `limit`). Recoverable on retry. No new failure modes introduced by the feature.

---

## Reproduction

Aggregate against v4 task definitions (v5 reuses them):

```bash
GRAPH_VERSION=v5 python3 benchmarks/graph/scripts/aggregate.py \
  --runs benchmarks/graph/v5/runs \
  --tasks benchmarks/graph/v4/tasks
```

Sweep command that produced this round:

```bash
GRAPH_VERSION=v4 python3 benchmarks/graph/scripts/sweep.py \
  --provider codex --arms graph-only --n 3 \
  --tasks reverse-export-orbit-error const-value-extraction module-surface-orbit-mcp
```

(Sweep targeted v4's runs/ directory; transcripts moved into `v5/runs/` after collection.)

---

## Status

This is the closing round. See [`benchmarks/graph/RESULTS.md`](../RESULTS.md) for the cross-round synthesis.
