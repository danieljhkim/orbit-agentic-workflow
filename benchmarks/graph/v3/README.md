# graph/v3 — LIVING

**Status:** LIVING — round 3 in progress. Fixtures and run records under this directory are mutable.

- Fixtures: [`./tasks/`](./tasks/)
- Staging for in-progress sweep data: [`./runs/`](./runs/) (gitignored)
- Shared scripts: [`../scripts/`](../scripts/) (used by all versions; copied into `v3/scripts/` at freeze)
- Shared harness overview + commands: [`../README.md`](../README.md)
- Most-recent frozen report: [`../v2/RESULTS.md`](../v2/RESULTS.md) (codex-only; see [`../v2/METHOD.md`](../v2/METHOD.md) for why)

Running a single cell against v3:

```bash
benchmarks/graph/scripts/run.sh graph-only locate-agentruntime 1 --provider claude
```

Or via the Makefile (defaults to `GRAPH_VERSION=v3`):

```bash
make -C benchmarks graph-run GRAPH_PROVIDER=claude GRAPH_ARM=graph-only GRAPH_TASK=locate-agentruntime
```

See [`../../CONVENTIONS.md`](../../CONVENTIONS.md) for the freeze procedure when round 3 concludes.
