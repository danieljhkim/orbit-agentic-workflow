# Graph Latency Benchmark v2

**Status: LIVING.** Round 2 is in progress. Inputs (`corpora.yaml`,
`tasks/queries.yaml`) start as a copy of v1's; the measurement change that
justifies cutting v2 will be recorded in `METHOD.md §Delta vs v1` once it
lands.

- Method: [`METHOD.md`](./METHOD.md)
- Results: [`RESULTS.md`](./RESULTS.md) (placeholder until first v2 sweep)
- Run records: [`runs/`](./runs/) (gitignored until v2 freeze)

The v1 frozen baseline is at [`../v1/`](../v1/). v2 must change at least one
of (fixtures, harness, system-under-test pin, interpretive frame) per
[`../../CONVENTIONS.md`](../../CONVENTIONS.md) §When to cut a new version.

## Candidate v2 changes (parked from v1's recommendations)

- **Drop `graph.history` from the matrix** (deprecated; 100% errors in v1; no info value).
- **Tighten `queries.yaml` seed=3** so `graph.callers`/`graph.refs` only see `symbol:` selectors. Removes 6 noise cells.
- **Pin indexer parallelism** via env var; record `host.parallelism_pin` in every record.
- **Sync + drop OS page caches** before each `build-cold` cell to remove the cold/warm-cache confounder.
- **Add a second corpus per language** (e.g. flask alongside django) to disambiguate corpus-specific findings.
- **Embed build SHA in `orbit --version`** so the harness records the binary's true source instead of the harness-checkout proxy. Once available, drop the proxy in run.py.
- **Add a Rust corpus** (e.g. `tokio-rs/tokio` or `rust-lang/rustfmt`) so `graph.deps` and `graph.implementors` actually run.
