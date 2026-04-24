# graph/v1 — FROZEN

**Status:** FROZEN snapshot of round 1, committed 2026-04-22 at SHA `9d42ccea`. Do not modify files in this directory.

- Report: [`RESULTS.md`](./RESULTS.md)
- Method / scope / caveats: [`METHOD.md`](./METHOD.md)
- Open issues at freeze: [`ISSUES.md`](./ISSUES.md)
- Shared harness overview: [`../README.md`](../README.md)
- Versioning rules: [`../../CONVENTIONS.md`](../../CONVENTIONS.md)

v1 uses the shared scripts at [`../scripts/`](../scripts/) — the aggregator is expected to remain backward-compatible with v1's record schema. To reproduce v1's tables:

```bash
make -C benchmarks graph-aggregate GRAPH_VERSION=v1
```
