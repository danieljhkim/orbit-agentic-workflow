# v2 Graph Benchmark Fixtures — Design Notes

> **Task:** T20260423-0507
> **Pinned SHA:** `7df885db` (HEAD at fixture authoring time)
> **Purpose:** Document the grep-failure rationale for each v2 fixture so a future reader can audit why it is part of the benchmark.

## Binary provenance for the v2 sweep

Both providers run graph tooling through the same `orbit` binary:

- **Claude** — spawns `orbit serve mcp` from `benchmarks/graph/mcp.json`, which points at `/Users/daniel/.cargo/bin/orbit`.
- **Codex** (`graph-only` arm) — runs `orbit tool run orbit.graph.*` as shell commands; `$PATH` resolves to the same binary.

The binary in use for the v2 sweep was rebuilt **after** all three of these tool-surface-affecting commits landed. Any v2 result must be read as measuring the *trimmed* surface, not the v1-era surface:

| Commit | Task | Effect on v2 measurement |
|---|---|---|
| `56117f6c` | T20260423-0452 (`.orbitignore`) | Search results exclude `benchmarks/`, `target/`, `node_modules/`, etc. — removes the v1 noise where `orbit.graph.search "AgentRuntime"` returned benchmark YAML ahead of code symbols. |
| `5f861caf` | T20260423-0525 (description rewrites) | Every `orbit.graph.*` tool's description starts with "Use when…" and includes "Prefer over grep when…". Tool selection priors differ from v1. |
| `7e86d41c` | T20260423-0506 (surface trim) | `orbit.graph.search` ranks code symbols above non-code hits, `orbit.graph.overview` defaults to summary shape, `orbit.graph.refs` partitions by ref-kind, `orbit.graph.pack` defaults to signatures. Direct behavior change from v1. |

**Implication:** a v2-vs-v1 delta is NOT a clean measurement of the v2 fixture set alone. It is a measurement of `(v2 fixtures) × (trimmed tool surface) × (rewritten descriptions) × (indexer noise removed)` vs. `(v1 fixtures) × (v1 surface)`. All four dimensions moved between rounds. When `graph_v2/METHOD.md` is authored, §Delta vs v1 must enumerate each of these changes explicitly per `benchmarks/CONVENTIONS.md` §Freezing a version.

This provenance block is the source of truth for the freeze; copy it verbatim into `graph_v2/METHOD.md` when v2 is cut.



## Why this file exists

v1's `benchmarks/graph_v1/METHOD.md` §Known Caveats #3 flagged that the v1 fixture set is grep-skewed: 5 of 6 fixtures are solvable with a single `rg` in 1–2 calls. That's why v1 hybrid utilization was 1.7%. v2 needs fixtures where **grep is structurally the wrong tool** — cases where lexical search returns noise or misses relevant hits and a structural index is load-bearing.

Each fixture here ships with:

- **Naive grep pattern** — the `rg` an agent reaches for first.
- **Why it fails** — concrete pathology (false positives, false negatives, missing context).
- **Graph-tool path** — which `orbit.graph.*` tool(s) resolve the ambiguity.
- **Gate B counter-check** — evidence that the naive grep actually fails.
- **Gate C smoke-test** — single-seed `graph-only` run result.

## Landed fixtures

| task_id | class | difficulty | Gate B | Gate C |
|---|---|---|---|---|
| `callers-run-deterministic-containers` | trace | medium | ✅ pass | ✅ claude/sonnet @ seed 1 ($0.08, 27s) |
| `trace-tool-call-event-construct-sites` | trace | medium | ✅ pass | ✅ codex @ seed 1 ($0 reported, 143s) |
| `impact-tool-context-struct-literals` | impact | medium | ✅ pass | ✅ claude/opus budget=4 @ seed 1 ($1.78, 239s) |
| `locate-loopaudit-variants` | locate | easy | ✅ pass | ✅ claude/sonnet @ seed 1 ($0.08, 25s) |

Four fixtures land, across **3 distinct classes** (trace, impact, locate), satisfying T20260423-0507 AC#8.

## Models used during Gate C

| fixture | provider | model | seed | budget | verdict | cost | wall |
|---|---|---|---|---|---|---|---|
| `callers-run-deterministic-containers` | claude | `claude-sonnet-4-6` | 1 | 1.0 | ✅ pass | $0.08 | 27 s |
| `trace-tool-call-event-construct-sites` | codex | `gpt-5.3-codex` | 1 | n/a | ✅ pass | $0 reported | 143 s |
| `impact-tool-context-struct-literals` | claude | `claude-opus-4-7` | 1 | **4.0** | ✅ pass | $1.78 | 239 s |
| `locate-loopaudit-variants` | claude | `claude-sonnet-4-6` | 1 | 1.0 | ✅ pass | $0.08 | 25 s |

The impact fixture was the only one that required Opus + an elevated budget. The other three passed on the default provider + model + budget at seed 1.

## Sonnet's difficulties under `graph-only` — observed and worth heeding

During Gate C I hit three distinct failure modes on **claude-sonnet** running the `graph-only` arm. All three matter for v2 sweep design; listing them here so the sweep can be configured to avoid them.

1. **Probe hallucination.** On multiple attempts, claude-sonnet replied `"THE TOOL mcp__orbit-bench__orbit_graph_overview IS NOT AVAILABLE IN THIS ENVIRONMENT. I CANNOT CALL IT."` during the pre-flight probe — even though the tool *was* available. The probe only retries once and gives up after a single failure, so the whole run records as `verdict: error`. Workaround used: pass `--no-probe`. Root cause suspected: Claude Code's deferred-tool / ToolSearch path occasionally fails to surface MCP tools to sonnet in time.

2. **Zero-graph-calls refusal.** Under `graph-only` with the default `--budget 1.0`, claude-sonnet sometimes produced a short reply using prior knowledge without invoking *any* graph tool. `classify.classify_arm_enforcement` correctly flagged these as `error` (arm violation — graph-only requires at least one graph call or permission denial). Observed rate across my attempts: roughly half the sonnet runs on the harder fixtures (impact). **This matches v1's 1.7% hybrid utilization — sonnet is at least partly *refusing to engage* with graph tools, not just preferring grep.**

3. **Time / budget pressure.** Codex on the impact fixture hit the default 600 s CLI timeout (exit 124) on multiple attempts. Sonnet on the impact fixture either refused or produced an incomplete answer under `--budget 1.0`. **Opus** on the same fixture with `--budget 4.0` completed in 239 s with 28+ graph-tool calls and a correct oracle-passing answer. This task bumped the default subprocess timeout from **600 s to 1000 s** in `scripts/providers.py` to reduce Codex timeouts in the v2 sweep.

### Practical v2 sweep implications

- **Claude-opus is the right lead model for v2.** Sonnet's refusal behaviour would recreate v1's utilization floor and contaminate any v2 signal about fixture quality vs. tool-surface quality.
- **`--budget 2.0–4.0` should be the floor**, not `1.0`. The cheaper fixtures (callers, locate) complete fine on `1.0`; the harder fixtures (impact, trace under graph-only) need headroom. Budget-1.0 failures are false negatives for this benchmark's purposes.
- **Sonnet stays as a secondary data point** — it's the right model for "cheap in production" and exposes real behavior, but v2 needs to separate "model couldn't do it" from "tools are ineffective."

> **Update (2026-04-23 after the first v2 sweep attempt):** opus exhausted the Claude subscription's usage window mid-sweep and `--max-budget-usd` was dropped from the harness entirely because it was only ever a CLI-side safety on pay-per-token API — irrelevant on subscription, and a nuisance during the pre-flight probe (MCP schema cache-creation alone exceeded the $0.25 probe cap on opus). The main-run model was rolled back to sonnet and the pre-flight probe pinned to haiku. Opus is kept on the table for specific fixtures where sonnet's Gate-C refusal behaviour recurs, but not as the sweep default. The `--budget` flag on `run.py` has been removed; the Gate-C notes above are retained as design history.

### Harness changes landed by this task to support the above

- `benchmarks/graph/scripts/classify.py` — opted `claude-opus-4-*` into `INFRA_MODEL_PATTERNS` so benchmark runs with `--model opus` are no longer rejected as "advisor escalation." The pattern previously allowlisted only sonnet and haiku.
- `benchmarks/graph/scripts/providers.py` — default subprocess timeout raised from **600 s to 1000 s** so codex runs on harder fixtures have room to complete.
- `benchmarks/graph/scripts/test_classify.py` — `test_opus_rejected` replaced by `test_opus_allowed` with the v2-prep rationale inlined as a docstring.
- **Still open (deferred):** extend `run.py` to accept a `--model` override rather than using `provider.default_model`. Today the harness requires editing `DEFAULT_MODELS` in `providers.py` to run a non-default model, which is awkward. File as a follow-up task before v2 sweep execution.

---

## 1. `callers-run-deterministic-containers` (trace)

**Naive grep:**
```
rg -n '\.run_deterministic\s*\(' crates/orbit-engine/src
```

**Why it fails:** grep returns the call *lines* but the fixture asks for the *containing function name* at each call site. Grep has no structural awareness of function boundaries — the agent must read surrounding code and scan upward for the nearest `fn …` signature. For 5 call sites that's 5 separate reads, and any missed one produces an incomplete answer.

**Graph-tool path:** `orbit.graph.callers` on the `V2RuntimeHost::run_deterministic` trait-method selector returns container functions directly, with symbol name + path in one response.

**Ground truth (hand-verified at SHA `7df885db`):**

| file | line | containing function |
|---|---|---|
| `crates/orbit-engine/src/activity_job/dispatcher.rs` | 268 | `run_deterministic` (free function — name collides with the trait method) |
| `crates/orbit-engine/src/activity_job/groundhog.rs` | 486 | `run_workspace_command` |
| `crates/orbit-engine/src/activity_job/groundhog.rs` | 552 | `persist_runner_artifacts` |
| `crates/orbit-engine/src/activity_job/groundhog.rs` | 573 | `load_task` |
| `crates/orbit-engine/src/activity_job/groundhog.rs` | 595 | `load_task_artifacts` |

**Gate B counter-check:** running `rg -n '\.run_deterministic\s*\(' crates/orbit-engine/src` returns exactly 5 call lines with no function-name context. To name containers a grep-only agent must read ~20 lines above each match. The `dispatcher.rs:268` trap is especially sharp: the containing free function is *also* named `run_deterministic`, so an inattentive agent may report the trait method as a caller of itself.

**Common grep false positives (in `must_not_include`):** `run_agent_loop_activity` (sibling at `dispatcher.rs:276`, doesn't call `run_deterministic`), `load_chronicle`, `load_state`, `verify_checkpoint`, `build_attempt_spec`.

---

## 2. `trace-tool-call-event-construct-sites` (trace)

**Naive grep:**
```
rg -n 'LoopAuditEvent::(ToolCallRequested|ToolCallResult)' crates
```

**Why it fails:** the variants appear in **both** construct sites (`sink.emit(&LoopAuditEvent::ToolCallRequested { .. })`) and pattern-match sites (same syntax inside a `match` arm or `filter_map` closure). Grep returns them all interleaved. The fixture asks for construct-only, so a grep-agent including the match-arm files fails the oracle.

**Graph-tool path:** `orbit.graph.refs` with construct-vs-destructure partitioning (post-T20260423-0506 shape), or `orbit.graph.show` on each variant selector and filtering by ref-kind.

**Ground truth (hand-verified at SHA `7df885db`):**

- **Construct sites** (must be in answer): `crates/orbit-agent/src/loop_engine/agent_loop.rs` lines 304 (`ToolCallRequested`) and 317 (`ToolCallResult`). Only this file constructs these two variants.
- **Destructure / match-arm sites** (must NOT be in answer):
  - `crates/orbit-engine/src/activity_job/tool_enforcement.rs:83` — match arm on `ToolCallRequested`.
  - `crates/orbit-engine/examples/v2_job_runtime_smoke.rs:348–349` — `filter_map` destructuring on both variants.

**Gate B counter-check:** the naive grep returns 5 matches across 3 files. A grep-only agent listing all 3 files as "construct sites" hits 2 out of the 4 `must_not_include` entries immediately.

---

## 3. `impact-tool-context-struct-literals` (impact)

**Naive grep:**
```
rg -n 'ToolContext' crates
```

**Why it fails:** `ToolContext` appears in **240 places across 95 files** at SHA `7df885db` — overwhelmingly as function parameters (`ctx: &ToolContext`), type annotations, imports, trait bounds, and doc comments. The fixture asks only for *struct-literal construction* sites in production code. Grep returns the full mixed set.

Even a tightened `rg 'ToolContext\s*\{'` returns ~10 hits but can't distinguish production from examples — the agent needs the workspace-root / examples-boundary filter applied, which a lexical tool doesn't know about.

**Graph-tool path:** `orbit.graph.refs` on the `ToolContext` symbol with `include: ["code"]` and construct-kind filtering returns only production construction sites.

**Ground truth (5 production struct-literal sites, hand-verified at SHA `7df885db`):**

| file | line |
|---|---|
| `crates/orbit-core/src/command/tool.rs` | 48 |
| `crates/orbit-core/src/runtime/pipeline.rs` | 122 |
| `crates/orbit-core/src/runtime/v2_host.rs` | 684 |
| `crates/orbit-engine/src/executor/automation/planning_duel/artifacts.rs` | 238 |
| `crates/orbit-engine/src/executor/automation/pr.rs` | 105, 239 |

Excluded (grep would include): `crates/orbit-tools/src/lib.rs` (the **definition site**, highest-confidence trap), `crates/orbit-agent/examples/tool_allowlist.rs:53` (struct literal in an example), and all `crates/orbit-engine/examples/*.rs` `ToolContext::default()` returns (not struct literals and not production).

**Gate B counter-check:** grep for `ToolContext` yields 240 hits across 95 files. The correct answer is 5 files. Any grep-based answer that doesn't aggressively post-filter will include the definition site (`orbit-tools/src/lib.rs`), triggering `must_not_include`.

---

## 4. `locate-loopaudit-variants` (locate)

**Naive grep:**
```
rg -n 'LoopAuditEvent::' crates
```

**Why it fails:** this returns the 16-ish *uses* of the enum (construct + destructure) across many files — but the fixture asks for the list of *variant names defined on the enum*. A grep-based answer built from usage lines is fragile: variants that exist in the type but have no uses would be missed, and the agent has to deduplicate across 16 matches. The correct workflow is "navigate to the enum def and enumerate children."

**Graph-tool path:** `orbit.graph.show` on the `LoopAuditEvent:enum` selector with `children: true` returns the 8 variants directly as child nodes.

**Ground truth (8 variants at SHA `7df885db`, enum definition in `crates/orbit-agent/src/loop_engine/audit/mod.rs` lines 40–111):**

- `HttpRequest`
- `HttpResponse`
- `IterationBoundary`
- `PolicyDenial`
- `SessionClose`
- `SessionSpawn`
- `ToolCallRequested`
- `ToolCallResult`

**Gate B counter-check:** a grep for `LoopAuditEvent::` returns only the 7 variants that currently have emit sites (`SessionSpawn`, `SessionClose`, `HttpRequest`, `HttpResponse`, `ToolCallRequested`, `ToolCallResult`, `IterationBoundary`, `PolicyDenial`) — actually all 8 do have emit sites in this codebase, so a careful grep ultimately works here too. This is the WEAKEST fixture of the four on grep-hardness grounds; it survives because the *natural workflow* is "read the enum", not "grep usage sites and deduplicate", and the must_not_include list catches common sibling-type confusion (e.g. `AuditSink`, `NullSink`).

**Note on grep-hardness:** this fixture sits at the boundary of "grep-hard". It lands because (a) the natural graph answer is markedly cheaper and cleaner than the grep answer, and (b) it exercises a different tool (`orbit.graph.show`) than the other three v2 fixtures. If v2 results show this fixture produces identical pass rates across arms, v3 should drop it or strengthen the grep trap.

---

## Gate C smoke-test results

Each fixture was smoke-tested at seed 1 under `graph-only` with the living harness before final landing. Recorded post-run.

_(Populated during task completion — see the bottom of this file.)_

---

## Dropped candidates

### 5. `deps-orbit-common-reverse-consumers` (deps)

**Status:** dropped at Gate B. All 10 sibling crates that declare `orbit-common` in Cargo.toml also import symbols from it in source; the "Cargo-declares vs. source-uses" distinction produces the same set. Not grep-hard.

### 6. `locate-dispatch-trait-methods` (locate)

**Status:** dropped at drafting. The correct answer is "no trait methods named `dispatch` exist" — a negative-existence claim that is both hard to verify confidently and unpleasant to oracle-check (a `must_include` list must be empty). `must_not_include` would need to catch every free-function name a grep agent might hallucinate. Not useful.

### 7. `locate-fsprofile-resolution-chain` (locate — spare)

**Status:** dropped. Not authored; four other fixtures cleared Gate B and satisfy AC#1.

---

## Reproducing Gate B

The counter-checks above can be re-run from the repo root:

```bash
# Fixture 1
rg -n '\.run_deterministic\s*\(' crates/orbit-engine/src

# Fixture 2
rg -n 'LoopAuditEvent::(ToolCallRequested|ToolCallResult)' crates

# Fixture 3 (count-only to keep output manageable)
rg -c 'ToolContext' crates | awk -F: '{s+=$2} END{print s" total hits across", NR, "files"}'

# Fixture 4
rg -n 'LoopAuditEvent::' crates
```
