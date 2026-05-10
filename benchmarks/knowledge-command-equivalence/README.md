# Knowledge Command Equivalence Harness

This harness captures byte-stable `orbit.graph.*` tool JSON for a prepared fixture workspace and compares a later run against that baseline. It was added with [T20260510-5] to make command-surface refactors reviewable without embedding tool-envelope dependencies in `orbit-knowledge`.

Typical use:

```bash
benchmarks/knowledge-command-equivalence/run.sh capture /path/to/fixture /tmp/orbit-graph-baseline
benchmarks/knowledge-command-equivalence/run.sh compare /path/to/fixture /tmp/orbit-graph-baseline
```

The fixture workspace should already contain representative code, docs, config, and graph write-tool targets such as `src/equivalence_write.rs`. The script copies the fixture to a temporary directory before running mutation cases. It runs through `orbit tool run` so it exercises the same public tool envelope agents use.
