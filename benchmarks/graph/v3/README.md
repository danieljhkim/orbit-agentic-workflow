# graph/v3 — LIVING

**Status:** LIVING — round 3 in progress. Fixtures and run records under this directory are mutable until freeze.

- Method + pre-registered disposition: [`METHOD.md`](./METHOD.md)
- Fixtures: [`./tasks/`](./tasks/)
- Staging for in-progress sweep data: [`./runs/`](./runs/) (gitignored)
- Shared scripts: [`../scripts/`](../scripts/) (used by all versions; copied into `v3/scripts/` at freeze iff the aggregator ever breaks backward-compat)
- Shared harness overview: [`../README.md`](../README.md)
- Most-recent frozen report: [`../v2/RESULTS.md`](../v2/RESULTS.md) — see [`../v2/METHOD.md`](../v2/METHOD.md) for why v2 is codex-only and why v3 is the last round.

Running a single cell against v3:

```bash
GRAPH_VERSION=v3 python3 benchmarks/graph/scripts/run.py \
  --provider claude --arm hybrid --task locate-agentruntime --seed 1
```

Or via the Makefile (defaults to `GRAPH_VERSION=v3`):

```bash
make -C benchmarks graph-run GRAPH_PROVIDER=claude GRAPH_ARM=hybrid GRAPH_TASK=locate-agentruntime
```

See [`../../CONVENTIONS.md`](../../CONVENTIONS.md) for the freeze procedure when round 3 concludes.
