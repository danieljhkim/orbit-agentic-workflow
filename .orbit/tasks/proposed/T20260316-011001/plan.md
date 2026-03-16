# Remove Job YAML Timestamp Fields Plan

**Goal:** Stop persisting `created_at` and `updated_at` in job YAML artifacts while preserving the behavior Orbit still needs from in-memory job metadata.
**Scope:** Job store serialization/deserialization, CLI/runtime expectations, and regression coverage around persisted job artifacts.
**Assumptions:** Job timestamps can remain part of runtime/domain structs if needed, but they should no longer be emitted into the YAML artifact contract.
**Risks:** Existing tests or readers may assume those keys exist, and older persisted job files may need an explicit compatibility decision.

## Task 1: Remove timestamps from the persisted job YAML contract

**Files:**
- Modify: `orbit-store/src/file/job_store.rs`
- Modify: any job persistence contract structs or helpers directly involved in YAML serialization/deserialization

**Steps:**
1. Identify where job YAML documents are serialized and deserialized.
2. Remove `created_at` and `updated_at` from the persisted YAML shape.
3. Keep any internal/runtime timestamp handling only where Orbit still genuinely needs it.
4. Decide and document whether older job YAML files with those fields should be tolerated or rejected.

**Done When:**
- Newly written job YAML artifacts no longer contain `created_at` or `updated_at`.
- Orbit can still read/write job artifacts according to the intended contract.

## Task 2: Update tests and user-facing expectations

**Files:**
- Modify: job store tests covering YAML round-trips
- Modify: CLI or runtime tests that assert on job YAML contents
- Modify: any docs or bundled artifacts that show these fields

**Steps:**
1. Update regression tests to assert that persisted job YAML omits the timestamp keys.
2. Adjust any fixtures, examples, or docs that still include them.
3. Run focused job-store coverage and broader workspace validation.

**Done When:**
- Tests verify the new YAML contract and no stale expectations remain.

## Final Verification
- `cargo test -p orbit-store job_write_read_roundtrip_preserves_all_fields -- --nocapture`
- `cargo test --workspace`