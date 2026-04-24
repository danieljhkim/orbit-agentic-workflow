# Knowledge Graph Benchmarks

Measures how much navigation budget an agent spends solving the same task under different tool surfaces. The benchmark compares repeatable experiment cells, not one-off anecdotes.

See [docs/design/knowledge-graph/](../../docs/design/knowledge-graph/) for the graph itself.

## Versions

Rounds are numbered; each round is a complete sweep campaign (fixtures + harness + run records + report). Frozen snapshots are immutable; the living version is where the next round is being developed.

| Version | Status | Report | Method |
|---|---|---|---|
| [v1](./v1/) | FROZEN (2026-04-22) | [RESULTS.md](./v1/RESULTS.md) | [METHOD.md](./v1/METHOD.md) |
| [v2](./v2/) | FROZEN (2026-04-23) | [RESULTS.md](./v2/RESULTS.md) | [METHOD.md](./v2/METHOD.md), [V2_FIXTURES.md](./v2/V2_FIXTURES.md) |
| [v3](./v3/) | LIVING (round 3 in progress) | — | — |

The [`../CONVENTIONS.md`](../CONVENTIONS.md) document governs version freezing, directory shape, and report structure. Version-specific deltas (fixture changes, harness changes, delta vs the previous round) live in that version's `METHOD.md`.

A null-result evidence log for the cross-round question *"do agent-facing graph tools earn their token cost?"* lives at [`docs/design/knowledge-graph/5_null_result.md`](../../docs/design/knowledge-graph/5_null_result.md).

## Providers

Two CLI providers are supported:

| Provider | Navigation surface in this benchmark |
|---|---|
| `claude` | Native Claude Code tools plus Orbit MCP graph tools |
| `codex` | Native Codex `exec_command`; graph access is Phase 1 shell-driven via `orbit tool run orbit.graph.*` |

Codex does not currently use direct MCP graph calls in this harness. In Phase 1, its graph path is provider-native shell execution from the repo root.

## Arms

Each run locks the child session to one of three navigation modes:

| Arm | Claude behavior | Codex behavior |
|---|---|---|
| `no-graph` | `Read`, `Grep`, `Glob`; graph tools denied | shell-only filesystem navigation (`rg`, `ls`, `find`, focused reads); `orbit.graph.*` forbidden by prompt and classified as an error if used |
| `graph-only` | Orbit graph MCP tools only | shell-only, but the intended surface is `orbit tool run orbit.graph.*`; zero graph calls with zero denials is classified as an error |
| `hybrid` | filesystem tools plus graph MCP tools | shell-only with both graph commands and filesystem commands allowed |

Claude keeps native allowlists. Codex is provider-native in Phase 1, so the arm boundary is enforced through prompt steering plus post-run classification of observed commands.

## Running

All commands target the living version (v3) by default. To run against a frozen snapshot for reproduction, pass `GRAPH_VERSION=vN` to the Makefile targets.

Run a single cell:

```bash
benchmarks/graph/v3/scripts/run.sh graph-only locate-agentruntime 1 --provider claude
benchmarks/graph/v3/scripts/run.sh graph-only locate-agentruntime 1 --provider codex
make -C benchmarks graph-run GRAPH_PROVIDER=codex GRAPH_ARM=graph-only GRAPH_TASK=locate-agentruntime
```

Run a sweep:

```bash
python3 benchmarks/graph/v3/scripts/sweep.py --provider claude --n 5
python3 benchmarks/graph/v3/scripts/sweep.py --provider codex --n 5
make -C benchmarks graph-sweep GRAPH_PROVIDER=codex GRAPH_N=1 GRAPH_TASKS=trace-policy-denial-wiring
```

Useful flags:

- `--provider {claude,codex}` selects the child CLI.
- `--no-probe` skips the graph pre-flight check for graph-enabled arms.

Cost accounting: Claude records `total_cost_usd` in each run record; Codex records cost as `0.0` in Phase 1 because its JSON event stream does not expose spend. There is no per-run CLI-side budget cap — subscription plans surface exhaustion as a 400 at the API layer.

## Tasks

Fixtures live under each version's `tasks/` folder and pin:

- `commit_sha`: exact repo state to run against
- `prompt`: verbatim user message
- `oracle`: grading rule for the final assistant message

See [`v3/tasks/_schema.yaml`](./v3/tasks/_schema.yaml) for the schema.

## Outputs

Per-run records live under each version's `runs/` subtree:

```text
benchmarks/graph/<version>/runs/<provider>/<arm>/<task_id>/<seed>.json
benchmarks/graph/<version>/runs/<provider>/<arm>/<task_id>/<seed>.transcript.json
```

Each run record includes:

- `provider`, `requested_model`, `arm`, `task_id`, `seed`
- token counts, wall time, total cost, tool-call histogram
- normalized `model_usage`
- `verdict` in `{pass, fail, error}` plus a diagnostic
- transcript artifact paths

Sweep metadata lives under `benchmarks/graph/<version>/runs/_sweeps/<provider>/<sweep_id>/`.

## Aggregation

```bash
make -C benchmarks graph-aggregate                  # living version (v3)
make -C benchmarks graph-aggregate GRAPH_VERSION=v1 # frozen v1
```

Prints:

- a primary table grouped by `(provider, arm, task_class)`
- a secondary table grouped by `(provider, model, arm, task_class)`
- an error table for excluded runs

## Directory Layout

```text
benchmarks/graph/
├── README.md        # this file (shared across versions)
├── v1/              # FROZEN round 1
│   ├── README.md    # version-specific banner
│   ├── METHOD.md
│   ├── RESULTS.md
│   ├── mcp.json
│   ├── scripts/     # harness as it ran
│   ├── tasks/       # fixtures as they were graded
│   └── runs/        # per-cell records
├── v2/              # FROZEN round 2
│   └── ...
└── v3/              # LIVING
    └── ...
```
