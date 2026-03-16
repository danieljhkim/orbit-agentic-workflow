# Remove Activity YAML Timestamp Fields Plan

**Goal:** Stop persisting `created_at` and `updated_at` in activity YAML artifacts while preserving the behavior Orbit still needs from in-memory activity metadata.
**Scope:** Activity store serialization/deserialization, CLI/runtime expectations, and regression coverage around persisted activity artifacts.
**Assumptions:** Activity timestamps can remain part of runtime/domain structs if needed, but they should no longer be emitted into the YAML artifact contract.
**Risks:** Existing tests or readers may assume those keys exist, and older activity YAML files may need an explicit compatibility decision.

## Task 1: Remove timestamps from the persisted activity YAML contract

**Files:**
- Modify: `orbit-store/src/file/activity_store.rs`
- Modify: any activity persistence contract structs or helpers directly involved in YAML serialization/deserialization

**Steps:**
1. Identify where activity YAML documents are serialized and deserialized.
2. Remove `created_at` and `updated_at` from the persisted YAML shape.
3. Keep any internal/runtime timestamp handling only where Orbit still genuinely needs it.
4. Decide and document whether older activity YAML files with those fields should be tolerated or rejected.

**Done When:**
- Newly written activity YAML artifacts no longer contain `created_at` or `updated_at`.
- Orbit can still read/write activity artifacts according to the intended contract.

## Task 2: Update tests and user-facing expectations

**Files:**
- Modify: activity store tests covering YAML round-trips
- Modify: CLI or runtime tests that assert on activity YAML contents
- Modify: any docs or bundled artifacts that show these fields

**Steps:**
1. Update regression tests to assert that persisted activity YAML omits the timestamp keys.
2. Adjust any fixtures, examples, or docs that still include them.
3. Run focused activity-store coverage and broader workspace validation.

**Done When:**
- Tests verify the new YAML contract and no stale expectations remain.

## Final Verification
- `cargo test -p orbit-store activity_write_read_roundtrip_preserves_all_fields -- --nocapture`
- `cargo test --workspace`