# v2 — Method & Caveats

This file documents how the `graph_v2` sweep was conducted. The report itself is `RESULTS.md` in this directory. Conventions governing this document's shape are in `benchmarks/CONVENTIONS.md`.

## Harness git SHA at freeze

`7e86d41c99cdeda9d46da2574794cd44bc1a80c6`

The harness code, fixtures, and run records under `benchmarks/graph/v2/` reflect the state of the repository at this SHA. Per [`V2_FIXTURES.md`](./V2_FIXTURES.md) §"Binary provenance", three additional commits land earlier in the same series and are baked into the `/Users/daniel/.cargo/bin/orbit` binary the agents hit during the sweep:

- `56117f6c` — T20260422-0452 `.orbitignore` (excludes benchmark fixture noise from the graph).
- `5f861caf` — T20260423-0525 tool-description rewrite.
- `7e86d41c` — T20260423-0506 tool-surface trim.

A v2-vs-v1 delta is therefore measuring `(v2 fixtures) × (trimmed surface) × (rewritten descriptions) × (indexer noise removed)` against `(v1 fixtures) × (v1 surface)`. All four dimensions moved between rounds.

## Delta vs v1

- **Fixtures:** added 4 grep-hard fixtures ([T20260423-0507]): `callers-run-deterministic-containers`, `impact-tool-context-struct-literals`, `locate-loopaudit-variants`, `trace-tool-call-event-construct-sites`. Kept the 6 v1 fixtures unchanged. Total: 10 fixtures.
- **Tool surface:** trimmed to 8 agent-facing `orbit_graph_*` MCP tools ([T20260423-0506]).
- **Tool descriptions:** rewritten for selector-first, navigation-first phrasing ([T20260423-0525]).
- **Indexer noise:** `.orbitignore` now excludes `benchmarks/graph*/` from the knowledge-graph ([T20260422-0452]).
- **Harness code:** pre-flight probe pinned to haiku; `--max-budget-usd` flag removed from the Claude CLI invocation (subscription usage-window is enforced upstream); subprocess timeout raised from 600 s to 1000 s.
- **Codex model regressed unintentionally:** v1 ran `gpt-5.4`; v2 ran `gpt-5.3-codex`. Caveat #7 below explains the impact on the v2-vs-v1 delta.

## Scope

- **Providers run:** Codex (`gpt-5.3-codex`, via `codex exec`). **Claude was NOT run** — the subscription usage window exhausted during the v2 sweep attempts; see caveat #1 below. Note: this is the plain `gpt-5.3-codex` variant, not `-spark`; an earlier draft of this file listed `-spark` in error. Verified against the `requested_model` field in all 90 run records.
- **Arms:** `no-graph`, `graph-only`, `hybrid`.
- **Fixtures:** 10 (6 carried from v1 + 4 new — see [`V2_FIXTURES.md`](./V2_FIXTURES.md)).
- **Seeds:** 3 per (provider × arm × fixture) cell.
- **Total runs (codex-only):** 3 arms × 10 fixtures × 3 seeds = 90 cells.
- **Sweep seed:** (see `runs/_sweeps/codex/<sweep_id>/order.json`).
- **Sweep date:** 2026-04-23.

## Fixture list

The v1 six carried into v2 are listed in [`../graph_v1/METHOD.md`](../graph_v1/METHOD.md) §"Fixture list". The four new fixtures:

| fixture | class | difficulty | one-line purpose |
|---|---|---|---|
| `callers-run-deterministic-containers` | callers | medium | Enumerate the 5 containing functions at call-sites of `V2RuntimeHost::run_deterministic`. Deny-list covers sibling methods that don't invoke it. |
| `impact-tool-context-struct-literals` | impact | medium | List production files that construct `ToolContext {...}` literals (5). Deny-list covers example/test files. |
| `locate-loopaudit-variants` | locate | easy | Enumerate the 8 variants of the `LoopAuditEvent` enum. Deny-list covers sibling types (`AuditSink`, `NullSink`, etc.) that are not variants. |
| `trace-tool-call-event-construct-sites` | trace | medium | List files that CONSTRUCT `LoopAuditEvent::ToolCallRequested{...}` or `ToolCallResult{...}`, excluding files that only pattern-match. One correct file; 3 concrete grep false positives in the deny-list. |

Full rationale per fixture (including the grep-failure mode each is designed to expose) is in [`V2_FIXTURES.md`](./V2_FIXTURES.md).

## Known caveats

These are material to interpreting `RESULTS.md` and should be read before relying on any headline number.

1. **Claude was NOT run for v2.** Two separate attempts both hit the Claude subscription usage window before completing. The first attempt on opus burned the window quickly; the second attempt on sonnet still collided with residual throttling from rapid-fire subprocess invocations (60 cells × 2 calls each with pre-flight probe). Both attempts' partial artifacts were deleted before the codex-only sweep. **Every headline in `RESULTS.md` is codex-only.** Claude cross-check is deferred to a future round or addendum.
2. **Utilization remains the dominant signal.** Codex made zero graph-tool calls across 30 hybrid runs — identical to v1's 0/30. The fixture redesign did not flip the utilization pattern. The hybrid vs. no-graph comparison therefore remains a schema-overhead null result, not a graph-tool value measurement. See `RESULTS.md` §Tool-utilization audit.
3. **graph-only costs widened, not narrowed.** v1 recorded 1.2–2.2× token multiplier on graph-only; v2 recorded ≈2.6× at the median, ≈3× at p90. The grep-hard fixtures made structural navigation more expensive (more graph hops per answer) rather than cheaper. See `RESULTS.md` §graph-only cost table.
4. **One fixture (`impact-tool-context-struct-literals`) broke graph-only entirely** — 0/3 pass on graph-only, 3/3 on both no-graph and hybrid. Transcripts show the agent making the graph navigation attempts but the assembled answer misses the construction sites the oracle expects. Worth a per-transcript audit before v3 design.
5. **Codex cost is reported as $0.** Same as v1 — the Codex CLI does not emit billing. All USD figures are zero by construction.
6. **Tool-utilization audit is still ad-hoc.** [T20260423-0524] (aggregate utilization column) landed in v2-prep but was not executed against the v2 sweep before freeze. Counts in `RESULTS.md` were produced by a transcript scan at report time, as in v1.
7. **Codex model regressed from v1.** v1 ran `gpt-5.4`; v2 ran `gpt-5.3-codex` (older). The v2-vs-v1 codex delta therefore conflates (tool-surface trim × description rewrite × indexer noise × grep-hard fixtures × `.orbitignore`) with a model downgrade. The regression was unintentional — it came from a DEFAULT_MODELS change at harness-polish time that was not noticed during pre-flight.
8. **Codex command_failures rate was high.** 38 of 90 runs (42 %) emitted at least one `command_failures` entry, 157 failures total. The dominant classes:
    - 45× `error: store error: attempt to write a readonly database` — SQLite WAL writes rejected because codex ran with `--sandbox read-only` and `~/.orbit/` was not in `--add-dir`. This broke graph CLI calls that went through the orbit binary.
    - 13× `error: unexpected argument '--output' found` — the agent invented a `--output` flag for `orbit tool list` that doesn't exist. A pure CLI-surface hallucination; MCP tool schemas would prevent this class of failure entirely (v3 measures that).
    - 9× WAL-mode warnings (same root cause as #1).
    Together these inflated the codex cost/turn counts and polluted the graph-only / hybrid arms' reliability signal in ways the `verdict: pass/fail` column alone doesn't expose. See v3 METHOD for how both classes are addressed.

## Reproduction

v2 uses the shared harness at `benchmarks/graph/scripts/` — at freeze time the shared aggregator was verified to read v2's record schema correctly (numbers match this directory's `RESULTS.md`). To regenerate the tables:

```
make -C benchmarks graph-aggregate GRAPH_VERSION=v2
```

Or directly:

```
GRAPH_VERSION=v2 python3 benchmarks/graph/scripts/aggregate.py \
  --runs benchmarks/graph/v2/runs \
  --tasks benchmarks/graph/v2/tasks
```

This regenerates the primary + secondary tables. The tool-utilization audit in `RESULTS.md` is not regenerated by this command.

## Task References

- **[T20260422-0452]** — `.orbitignore` exclusion file for the indexer.
- **[T20260423-0506]** — Tool-surface trim (agent-facing `orbit_graph_*` set).
- **[T20260423-0507]** — v2 grep-hard fixture design.
- **[T20260423-0524]** — aggregate.py utilization column (v2-prep, not yet run).
- **[T20260423-0525]** — Tool-description rewrite.
- **[T20260423-0607]** — v3 pointer-only graph reads (planned, not yet run).

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
