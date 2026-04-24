# Benchmark Conventions

Rules for how Orbit benchmarks are laid out, versioned, and reported.

This document governs shape only — what directories exist, what files they contain, what sections a report must have. It does not prescribe what benchmarks measure; that's per-benchmark and lives in each benchmark's own `README.md` and fixture schema.

---

## Directory layout

Each benchmark lives under a single `benchmarks/<name>/` parent. Within it, every round — frozen or in-progress — gets its own `vN/` subdirectory. Files that are legitimately shared across rounds (the overview README, the living scripts, the MCP config) live at the benchmark root. Per-round content (fixtures, run records, report, method) lives inside `vN/`.

- `benchmarks/<name>/README.md` — **shared overview**. Describes the benchmark, providers, arms, how to run, output layout. Lists the versions with links to each.
- `benchmarks/<name>/scripts/` — **shared harness**. Edited as part of living-version development. Expected to stay backward-compatible with every frozen round's record schema so a single aggregator / classifier / token-normalizer handles all versions. If a round's work forces a back-compat break, that round snapshots `scripts/` into its own `vN/scripts/` at freeze time and the Makefile's `GRAPH_SCRIPTS` override points there for reproduction.
- `benchmarks/<name>/mcp.json` — shared transport config. Same back-compat expectation as scripts; snapshot into `vN/mcp.json` only if it diverges.
- `benchmarks/<name>/vN/` — per-round directory. Frozen for completed rounds, LIVING for the in-progress round.
  - `vN/README.md` — short per-round banner: status, links to METHOD/RESULTS.
  - `vN/METHOD.md` — scope, delta vs previous version, caveats, reproduction command.
  - `vN/RESULTS.md` — the report.
  - `vN/ISSUES.md` — open issues at freeze (optional).
  - `vN/tasks/` — fixture set as graded for round N.
  - `vN/runs/` — per-cell records and `_sweeps/<sweep_id>/` metadata.
  - `vN/scripts/` (optional) — only present if round N broke back-compat with the shared scripts at freeze time. Default is to rely on shared `../scripts/`.
  - `vN/mcp.json` (optional) — same rule as scripts.

Example (graph benchmark):

```
benchmarks/
├── Makefile                  # graph-* targets; GRAPH_VERSION=vN selects a version
├── CONVENTIONS.md            # this file
└── graph/
    ├── README.md             # shared overview; versions table
    ├── mcp.json              # shared MCP config
    ├── scripts/              # SHARED harness (sweep.py, aggregate.py, providers.py, tests, …)
    ├── v1/                   # FROZEN round 1
    │   ├── README.md         # version banner
    │   ├── METHOD.md
    │   ├── RESULTS.md
    │   ├── ISSUES.md
    │   ├── tasks/
    │   └── runs/             # per-cell records + _sweeps/…
    ├── v2/                   # FROZEN round 2
    │   └── …                 # same shape
    └── v3/                   # LIVING round 3
        ├── README.md
        ├── tasks/            # mutable — next-round fixture set
        └── runs/             # gitignored staging for in-progress sweep
```

Neither the living version nor the frozen versions carry their own `scripts/` or `mcp.json` under this convention — shared copies at the benchmark root serve all rounds. Frozen versions carry only round-specific content (tasks, runs, reports). If a future round breaks back-compat with the shared harness, it snapshots `scripts/` (and `mcp.json` if relevant) into its own `vN/` at freeze time; the Makefile's `GRAPH_SCRIPTS` override addresses that case.

Everything in a frozen `vN/` directory is *immutable* and represents "what round N actually measured, reproducibly." The shared `scripts/` + the living `vN/` together represent "what the next round will use."

---

## When to cut a new version

A version captures a **sweep campaign** — the report plus every input that produced it. Start a new version when any of these changes vs the previous frozen version:

1. **Fixture set changed** — tasks added, removed, or materially edited.
2. **Harness code that affects measurement changed** — scoring, token normalization, arm enforcement, classifier.
3. **Model or provider lineup changed**.
4. **The interpretive frame changed** — e.g. adding a tool-utilization audit that reframes the headline.

Do NOT cut a new version for:

- Stochastic re-runs on the same code + fixtures (that's seed expansion; keep running and either update the current version if unfrozen, or note it as an addendum to the frozen version).
- Typo fixes in the report (append to `CORRECTIONS.md`, see §Immutability).
- Harness code changes that don't affect measurement (refactors, logging cleanups).

---

## Freezing a version

Freezing round N means: the living `vN/` becomes immutable, and a fresh `v(N+1)/` is scaffolded as the next living version.

1. Ensure the sweep is complete and `RESULTS.md` is written inside `benchmarks/<name>/vN/`.
2. Remove (or overwrite) the `.gitignore` in `benchmarks/<name>/vN/runs/` so the run records are committed. See §"Runs and git" below.
3. Author `benchmarks/<name>/vN/METHOD.md` (see required sections below).
4. Optionally prune staging artifacts (e.g. failed partial sweeps under `runs/_sweeps/` that are not part of vN).
5. Replace the per-version banner in `benchmarks/<name>/vN/README.md` with a frozen banner so opening it out of context makes the status obvious.
6. **Back-compat check**: confirm the shared `scripts/` and `mcp.json` still correctly read/reproduce `vN/runs/` (run the aggregator; numbers should match `RESULTS.md`). If not — i.e. this round introduced a record-schema change or a behavior change that can't be backported — snapshot `scripts/` (and `mcp.json` if relevant) into `vN/` and note the divergence in METHOD.md §Delta.
7. Scaffold the new living version `v(N+1)/`: copy `vN/tasks/` as the starting fixture set, create an empty `runs/` with a `.gitignore`, write a LIVING-status README.
8. Bump the default `GRAPH_VERSION` (or benchmark's equivalent) in `benchmarks/Makefile` to `v(N+1)`.
9. Update the versions table in `benchmarks/<name>/README.md` to include the newly frozen round and the new living round.

A freeze lands in a single commit separate from v(N+1) development, so the diff is easy to review: it's additive (new frozen `vN/scripts|mcp.json`, new `v(N+1)/` skeleton) plus the Makefile version bump.

## Runs and git

The two kinds of `runs/` have different commit rules:

- **Living `benchmarks/<name>/vN/runs/`** — ships with a `.gitignore` that excludes all contents except the `.gitignore` itself. This is a staging area for in-progress sweeps; every developer's half-finished runs should not flood the history.
- **Frozen `benchmarks/<name>/vN/runs/`** (after the round is frozen) — contents ARE committed. The run records (`<seed>.json`) and transcripts (`<seed>.transcript.json`) are the evidence behind `RESULTS.md`; without them the reproduction command in `METHOD.md` cannot execute, and the tool-utilization audit in `RESULTS.md` cannot be re-verified.

Before committing a new frozen version, scan transcripts for secrets and PII:

```
grep -rE "sk-ant-[A-Za-z0-9]{20}|sk-proj-[A-Za-z0-9]{20}|AKIA[A-Z0-9]{16}|Bearer [A-Za-z0-9]{20}" \
  benchmarks/<name>/vN/runs/
```

Matches are blockers. Env-var *names* (`ANTHROPIC_API_KEY` as a string, not a value) are acceptable. Absolute paths that leak a local username are acceptable if the same username already appears in `git log` (i.e. no new information is disclosed).

---

## Immutability

Once `benchmarks/<name>/vN/RESULTS.md` is committed, that directory is read-only by convention.

- **Factual correction** → append a dated entry to `benchmarks/<name>/vN/CORRECTIONS.md`. Do not mutate `RESULTS.md`, `METHOD.md`, fixtures, or runs.
- **Reinterpretation with the same evidence** → write a new `METHOD.md §Delta vs v(N-1)` in the *next* version, or a standalone `benchmarks/<name>/COMPARE-vN-vs-vM.md` at the shared level.
- **New evidence** → cut a new version.

If a frozen snapshot has to be mutated for a reason that doesn't fit the above (e.g. accidentally committed secrets), the commit message must call it out explicitly.

---

## `RESULTS.md` — required sections

Every `benchmarks/<name>_vN/RESULTS.md` MUST have, in order:

1. **Frontmatter** — task ID, sweep date, sweep seed(s), scope (providers × arms × fixtures × seeds), fixture list by name. One paragraph.
2. **Headline** — 3–6 bullets summarizing what the sweep showed. Lead with the strongest, most load-bearing finding. If a naive read of the tables would mislead, say so in the headline, not as reader inference.
3. **Tool-utilization audit** — per-arm counts of which tools agents actually called. A cost table without utilization is incomplete: if agents didn't use a tool, its cost measurement is about schema overhead, not tool value. Omit only if the benchmark structurally cannot vary tool surface.
4. **Primary table** — the provider × arm × task_class aggregate. Columns: `runs`, `pass_rate`, `median_total_tokens`, `p90_total_tokens`, `tokens_per_success`. Produced by `scripts/aggregate.py`.
5. **Cost (USD)** — per-arm totals for providers that report billing. State explicitly when a provider reports $0 due to a CLI limitation rather than free usage.
6. **Pass-rate breakdown** — every failed run listed by (provider, arm, fixture, seed) with the rejection reason.
7. **Re-interpretation** (encouraged) — spell out any reframing. If the raw tables say one thing and the utilization audit says another, reconcile them here.
8. **Hypothesis reconciliation** — pre-sweep hypotheses graded ✅ supported / ❌ falsified / ⏸ untested. If no pre-sweep hypotheses were filed, write "No pre-sweep hypotheses were filed." and move on.
9. **Recommendations** — actionable next steps derived from the evidence. Separate "change in the product" from "change in the next sweep."
10. **Methodology notes** — token-accounting convention, known caveats, reproduction command, any ad-hoc analyses not in `aggregate.py`.

Sections may be subdivided but must appear in this order.

---

## `METHOD.md` — required sections

Every `benchmarks/<name>_vN/METHOD.md` MUST have:

1. **Harness git SHA at freeze time** — so the report is reproducible from source.
2. **Delta vs v(N-1)** — fixture diff, harness diff, model diff, scope diff. If this is v1, write "v1 is the first frozen round; no prior version to diff."
3. **Fixture list with one-line purposes** — so future readers don't have to open each YAML.
4. **Known caveats** — things that would bias a naive reading of `RESULTS.md`.
5. **Reproduction command** — exact CLI to regenerate the primary tables from the frozen fixture + run snapshots.

---

## Addressing a frozen version in tooling

The top-level `benchmarks/Makefile` should accept a version variable (e.g. `GRAPH_VERSION`) that swaps `benchmarks/<name>/` for `benchmarks/<name>_vN/` in script paths. This lets:

```
make -C benchmarks graph-aggregate                      # against the living harness
make -C benchmarks graph-aggregate GRAPH_VERSION=v1     # against the v1 snapshot
```

without copying code or modifying scripts. Unset defaults to the living harness.

---

## Referencing a frozen version in docs, PRs, commits

- Cite `benchmarks/<name>_vN/RESULTS.md` plus the commit SHA at which the report landed. Never write "the benchmark results" or "RESULTS.md" without a version — those phrases refer to mutable current state and will rot.
- Orbit task summaries (`purpose`, `implementation`) that rely on benchmark evidence must include the version number.
- When a PR claims a performance or accuracy effect, it should either (a) cite a frozen version, or (b) include a new frozen version produced by the PR.
