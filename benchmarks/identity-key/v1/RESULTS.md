# Identity-Key Benchmark v1 Results

Task T20260508-2, run on May 8, 2026. Scope: Rust-only five-scenario baseline
against `orbit_knowledge::pipeline::run_build`; records are stored in
[`runs/`](./runs/).

## Headline

- `identity_key` is preserved when the path, qualified name, and kind stay fixed.
- Path changes are not preserved: both `rename` and `move` produce new keys and
  new IDs for the same Rust function names.
- Source and signature text changes do not change `identity_key` in v1.

## Primary Table

| Scenario | Mutation | identity_key preserved? | Notes |
|---|---|---|---|
| rename | `a.rs -> b.rs` | no | Path changed; `identity_key` includes the file location. |
| move | `src/a.rs -> src/sub/a.rs` | no | Directory changed; `identity_key` includes the file location for both `foo` and `bar`. |
| content_edit | edit `foo`'s body | yes | Body changed, but path, qualified name, and kind stayed fixed. |
| delete_recreate | rm + recreate same content | yes | After an intermediate deletion rebuild, recreating the same path/name/kind produced the same key. |
| signature_change | `u32 -> u64` | yes | Signature text changed, but current key derivation ignores signature/source hash. |

## Synthesis

The v1 "no" rows are `rename` and `move`; those are the direct inputs for
v2-bridge planning because a task-to-KG bridge cannot assume leaf identity
survives path changes on current `agent-main`. The `content_edit`,
`delete_recreate`, and `signature_change` rows preserve identity under today's
path/name/kind-based derivation.

## Methodology Notes

Reproduce with `make -C benchmarks identity-key-run`. See
[`METHOD.md`](./METHOD.md) for the exact build config and caveats.
