# graph/v2 — FROZEN

**Status:** FROZEN snapshot of round 2, frozen 2026-04-23. Do not modify files in this directory.

- Report: [`RESULTS.md`](./RESULTS.md)
- Method / scope / caveats: [`METHOD.md`](./METHOD.md)
- v2 fixture-design notes: [`V2_FIXTURES.md`](./V2_FIXTURES.md)
- Shared harness overview: [`../README.md`](../README.md)
- Versioning rules: [`../../CONVENTIONS.md`](../../CONVENTIONS.md)

v2 uses the shared scripts at [`../scripts/`](../scripts/). To reproduce v2's tables:

```bash
make -C benchmarks graph-aggregate GRAPH_VERSION=v2
```
