# Graph Latency Benchmark v1 Method

## Harness git SHA at freeze time

`f6097e0a119631728f76e09f3d82c73867cf1684`. Reproducing this report requires
the harness sources at this SHA AND a release-mode orbit binary built from it.

## Delta vs v0

v1 is the first frozen round; no prior version to diff.

(An earlier v1 round was attempted and discarded: its `orbit_sha` was a
harness-checkout proxy and the actual binary in PATH was a stale `cargo install`
of `orbit-cli v0.1.0` predating the recorded SHA. Numbers from that round were
unverifiable and have been deleted from history. The current v1 is the first
round whose binary identity matches the recorded SHA, achieved by an immediate
`cargo install --path crates/orbit-cli --force` from `f6097e0a` before the
sweep.)

## Corpus list

Three corpora at the `medium` (~250k LOC) tier. All corpora pinned at a
specific upstream SHA and fetched into `~/.cache/orbit-bench/<corpus>` by
`scripts/fetch.sh`.

| Corpus name     | Language   | Source                | LOC target |
|-----------------|------------|-----------------------|-----------:|
| `python-medium` | Python     | `django/django@5.1.2` |      ~280k |
| `java-medium`   | Java       | `google/guava@v33.4.8`|      ~150k |
| `ts-medium`     | TypeScript | `vuejs/core@v3.5.13`  |      ~150k |

(Concrete commit SHAs are recorded in [`corpora.yaml`](./corpora.yaml). The
medium tier targets ~250k LOC; larger tiers are deferred until the design is
known to handle 700k+ LOC corpora cleanly.)

TypeScript is included because `orbit-knowledge` parses it as a first-class
language and TS exercises pathologies neither Python nor Java cover well —
barrel re-exports (`export * from './x'`), `import type` vs value imports,
and conditional types.

## In-scope tools

All nine `orbit.graph.*` MCP tools, one cell per tool per corpus per phase:

- `orbit.graph.overview`
- `orbit.graph.search`
- `orbit.graph.callers`
- `orbit.graph.deps`
- `orbit.graph.refs`
- `orbit.graph.show`
- `orbit.graph.implementors`
- `orbit.graph.history`
- `orbit.graph.pack`

`graph.history`, `graph.deps`, and `graph.implementors` produce 100% errors
on this corpus set (deprecated; Rust-only; trait-only). They are recorded
faithfully so the language-applicability gap is visible from outside the
source. v2 may drop or guard them.

## Phases

- **build-cold** — full index of the corpus from a clean cache. One observation per corpus per seed.
- **build-incremental** — incremental rebuild after a controlled mutation (single appended line to one source file). One observation per corpus per seed.
- **query** — N=5 seeded calls of each tool against the built index. Distribution reported as p50/p90/p99.

## Per-cell record schema

Each `runs/<corpus>/<tool>/<phase>/<seed>.json` (or `<corpus>/_build/<phase>/<seed>.json`)
record has exactly these fields:

| Field                | Type    | Notes                                                                 |
|----------------------|---------|-----------------------------------------------------------------------|
| `corpus`             | string  | corpus name from the table above (e.g. `python-medium`)               |
| `tool`               | string  | `graph.<name>` for query phase; `null` for build phases               |
| `query_shape`        | string  | id derived from `v1/tasks/queries.yaml`; `null` for build phases      |
| `phase`              | string  | one of `build-cold`, `build-incremental`, `query`                     |
| `seed`               | integer | 1-indexed seed for query selection                                    |
| `wall_ms`            | integer | wall-clock duration of the measured operation in milliseconds         |
| `rss_peak_mb`        | integer | peak resident set size during the operation, in MiB                   |
| `result_size_bytes`  | integer | size of the JSON tool result; `null` for build phases                 |
| `result_count`       | integer | top-level result count from the tool; `null` for build phases         |
| `host`               | object  | `{ "cpu": str, "ram_gb": int, "os": str }`                            |
| `orbit_sha`          | string  | 40-char git SHA of the harness checkout (used as a proxy for binary's source — see Caveats) |
| `corpus_sha`         | string  | 40-char git SHA of the corpus checkout                                |

The aggregator reads only these fields. Adding new fields is allowed and
non-breaking; removing or renaming any field is a record-schema break and
requires a new round per [`../../CONVENTIONS.md`](../../CONVENTIONS.md) §When to cut a new version.

## Host disclosure rules

The harness records `host.cpu`, `host.ram_gb`, and `host.os` into every
record. `RESULTS.md` §Host/environment disclosure MUST state explicitly
whether all rows in the primary table came from a single host or were
aggregated across hosts.

For v1: aggregation across hosts is **not allowed** in the primary table.
Cross-host data may appear in a separate appendix table but never in the
headline.

## Known caveats

- **`orbit_sha` is a proxy.** The harness records the harness checkout's git
  HEAD as `orbit_sha`. This matches the running binary's source only when the
  sweep was preceded by an immediate `cargo install --path crates/orbit-cli
  --force` from that checkout — which v1 enforced. If a future round runs
  without that step, the recorded SHA may diverge from the binary's actual
  source. v2 candidate change: embed the build SHA in `orbit-cli` (via
  `build.rs` or `vergen`) so `orbit --version` exposes it and the proxy can
  be retired.

- **Cold-cache effects unpinned.** The harness deletes
  `<corpus>/.orbit/knowledge/` before each `build-cold` cell but does not
  clear the OS page cache. Page-cache state can shift `build-cold` numbers
  by ~2x. Java/Python build-cold p99 spreads (~30-50 ms) suggest cache
  effects are negligible at this corpus size, but the rule should be
  enforced before the design is trusted at the large tier.

- **Indexer parallelism unpinned.** Build-phase numbers are sensitive to
  thread-pool size. The v1 sweep used the OS-default; future rounds should
  pin `RAYON_NUM_THREADS` (or equivalent) and record `host.parallelism_pin`.

- **Single corpus per language.** A regression observed on django can't be
  disambiguated as "django-specific" vs "python-parser-wide" without a second
  python corpus. Same for Java and TypeScript. Future rounds may broaden.

- **Subprocess startup cost is bundled into `wall_ms`.** Each cell forks a
  fresh `orbit` binary, paying ~30-100 ms of process startup. For sub-200 ms
  TS query cells, that overhead is a meaningful share of the measurement.
  v2 candidate: an MCP-server-based harness if sub-100 ms targets become
  real budgets.

- **Build-incremental mutation is a single appended line.** The cheapest
  possible delta. A larger delta would presumably be slower; the universal
  "incremental > cold" finding is the dominant signal, and the absolute
  numbers should be read as a floor, not a ceiling.

## Reproduction command

```bash
GRAPH_LATENCY_VERSION=v1 make -C benchmarks graph-latency-fetch
GRAPH_LATENCY_VERSION=v1 make -C benchmarks graph-latency-sweep
GRAPH_LATENCY_VERSION=v1 make -C benchmarks graph-latency-aggregate
```
