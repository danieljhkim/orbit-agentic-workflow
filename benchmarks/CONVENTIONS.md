# Benchmark Conventions

Rules for how Orbit benchmarks are laid out, versioned, and reported.

This document governs shape only — what directories exist, what files they contain, what sections a report must have. It does not prescribe what benchmarks measure; that's per-benchmark and lives in each benchmark's own `README.md` and fixture schema.

---

## Directory layout

Each benchmark owns two kinds of directories under `benchmarks/`:

- `benchmarks/<name>/` — the **living harness**. Mutable. This is where next-round development happens: script edits, fixture additions, in-progress sweep data.
- `benchmarks/<name>_vN/` — a **frozen snapshot** of round N. Immutable by convention. Contains the harness as it was, the fixtures as they were, the raw run records, and the report.

Example (graph benchmark):

```
benchmarks/
├── Makefile                   # targets per benchmark; supports GRAPH_VERSION=v1 to address a frozen snapshot
├── CONVENTIONS.md             # this file
├── graph/                     # living harness — v2-in-progress
│   ├── README.md              # overview of the current harness
│   ├── ISSUES.md              # open follow-ups against the harness (optional)
│   ├── mcp.json               # transport config
│   ├── scripts/               # sweep.py, aggregate.py, providers.py, tests, …
│   ├── tasks/                 # current fixture set
│   └── runs/                  # staging for in-progress or most-recent unpublished sweep
└── graph_v1/                  # FROZEN snapshot of round 1
    ├── README.md              # snapshot of harness README at freeze time
    ├── RESULTS.md             # the report for this round
    ├── METHOD.md              # scope, caveats, reproduction
    ├── ISSUES.md              # issues note at freeze (optional, per-round)
    ├── mcp.json
    ├── scripts/               # harness code exactly as it ran
    ├── tasks/                 # fixtures exactly as they were graded
    └── runs/                  # per-cell run records + `_sweeps/<sweep_id>/` metadata
```

Everything in the living directory is *mutable* and represents "what the next round will use." Everything in a `_vN` directory is *frozen* and represents "what round N actually measured, reproducibly."

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

1. Ensure the sweep is complete and `RESULTS.md` is written in the living directory's style.
2. `cp -r benchmarks/<name>/ benchmarks/<name>_vN/` — full snapshot.
3. **Remove or overwrite the `.gitignore` in `benchmarks/<name>_vN/runs/`** so the run records are committed. See §"Runs and git" below.
4. Author `benchmarks/<name>_vN/METHOD.md` (see required sections below).
5. Optionally prune staging artifacts from the snapshot (e.g. failed partial sweeps under `runs/_sweeps/` that are not part of vN).
6. Add a frozen banner to the snapshot's `README.md` so opening it out of context makes the status obvious.
7. Before the next round starts, clear `benchmarks/<name>/runs/` contents (the living `.gitignore` keeps it uncommitted either way).

A freeze lands in a single commit separate from v(N+1) work, so the diff is easy to review: it's purely additive (new `_vN/` directory) plus the version-bump Makefile hint.

## Runs and git

The two `runs/` directories have different commit rules:

- **Living `benchmarks/<name>/runs/`** — ships with a `.gitignore` that excludes all contents except the `.gitignore` itself. This is a staging area for in-progress sweeps; every developer's half-finished runs should not flood the history.
- **Frozen `benchmarks/<name>_vN/runs/`** — contents ARE committed. The run records (`<seed>.json`) and transcripts (`<seed>.transcript.json`) are the evidence behind `RESULTS.md`; without them the reproduction command in `METHOD.md` cannot execute, and the tool-utilization audit in `RESULTS.md` cannot be re-verified.

Before committing a new frozen version, scan transcripts for secrets and PII:

```
grep -rE "sk-ant-[A-Za-z0-9]{20}|sk-proj-[A-Za-z0-9]{20}|AKIA[A-Z0-9]{16}|Bearer [A-Za-z0-9]{20}" \
  benchmarks/<name>_vN/runs/
```

Matches are blockers. Env-var *names* (`ANTHROPIC_API_KEY` as a string, not a value) are acceptable. Absolute paths that leak a local username are acceptable if the same username already appears in `git log` (i.e. no new information is disclosed).

---

## Immutability

Once `benchmarks/<name>_vN/RESULTS.md` is committed, that directory is read-only by convention.

- **Factual correction** → append a dated entry to `benchmarks/<name>_vN/CORRECTIONS.md`. Do not mutate `RESULTS.md`, `METHOD.md`, fixtures, or runs.
- **Reinterpretation with the same evidence** → write a new `METHOD.md` section in the *next* version titled `## Delta vs v(N-1)`, or a standalone `benchmarks/<name>/COMPARE-vN-vs-vM.md` at the living level.
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
