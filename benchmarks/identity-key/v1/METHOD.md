# Identity-Key Benchmark v1 Method

## Harness Git SHA at Freeze Time

Production baseline: `daa8f9c693eb2413258d9daaa3f9a3787b559bf6`
(`agent-main` HEAD before T20260508-2 benchmark additions). The harness and v1
records are authored by T20260508-2 and should be cited by the commit that lands
this benchmark directory.

## Delta vs v0

v1 is the first round; no prior version exists.

## Fixture List

| Scenario | Purpose |
|---|---|
| `rename` | Rename `a.rs` containing `foo` to `b.rs`. |
| `move` | Move `src/a.rs` containing `foo` and `bar` to `src/sub/a.rs`. |
| `content_edit` | Edit `foo`'s body while keeping the function name and signature stable. |
| `delete_recreate` | Delete `a.rs`, rebuild, then recreate the same file contents and rebuild again. |
| `signature_change` | Change `foo`'s argument and return type from `u32` to `u64`. |

## Scope

The benchmark is Rust-only. Each scenario runs in a fresh temporary git
repository with an initial commit, then calls
`orbit_knowledge::pipeline::run_build` directly. The first build uses
`BuildConfig { incremental: false, ref_name: Some("identity-key-v1") }`; the
post-mutation build uses the same `repo_path`, `output_dir`, and ref name with
`incremental: true`.

`delete_recreate` has one additional incremental build between deletion and
recreation so the graph observes the missing leaf before the file returns.

## Known Caveats

- The records observe `identity_key` and `id` for selected Rust leaves only.
- The harness commits each mutation in the temporary git repo before the
  incremental build so git state stays clean and the ref name is stable.
- Rust function signature details are not part of the current `identity_key`
  derivation; v1 records the observed result rather than asserting a desired
  bridge policy.
- No production source under `crates/orbit-knowledge/src/` is changed by this
  benchmark.

## Reproduction Command

```bash
make -C benchmarks identity-key-run
```

The command overwrites exactly these records:

```text
benchmarks/identity-key/v1/runs/rename.json
benchmarks/identity-key/v1/runs/move.json
benchmarks/identity-key/v1/runs/content_edit.json
benchmarks/identity-key/v1/runs/delete_recreate.json
benchmarks/identity-key/v1/runs/signature_change.json
```
