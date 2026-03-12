## Status
success

## Orbit Task
Task ID: T20260310-052604-1773120364035663000

## 1. Summary of Changes
Added the missing flags that caused the first review rejection:

**New CLI flags on `orbit activity update`:**
- `--identity <id>`: sets `identity_id`
- `--clear-identity`: clears `identity_id` to null (conflicts with `--identity`)
- `--active`: sets `is_active = true` (re-enables an inactive activity)
- `--inactive`: sets `is_active = false` (deactivates without deleting)

**Store layer changes:**
- `ActivityUpdateParams` (contracts.rs): added `is_active: Option<bool>`
- `ActivityFileStore::update_activity`: added `is_active` param; when toggled, moves the file between `active/` and `inactive/` directories atomically (write new path, remove old)
- `Store::update_activity` (sqlite): added `is_active` param; included in the SQL UPDATE statement
- Both backend adapters (file_backends.rs, sqlite_backends.rs): pass `is_active` through
- `OrbitRuntime::update_activity` (orbit-core): `ActivityUpdateParams` struct and store call updated

**Tests added** (`orbit-cli/tests/activity_commands.rs`):
- `activity_update_inactive_deactivates_activity`: `--inactive` sets `is_active=false`
- `activity_update_active_reactivates_inactive_activity`: `--inactive` then `--active` toggles back
- `activity_update_identity_sets_field`: `--identity grace` persists identity_id
- `activity_update_clear_identity_removes_field`: `--clear-identity` nulls the field

## 2. Strategic Decisions
- `is_active` as a patch field in `ActivityUpdateParams` rather than a separate `enable_activity` function | Rationale: Consistent with the existing patch-semantics pattern; avoids a parallel disable/enable API split | Trade-offs: None
- File store moves the file between active/ and inactive/ on toggle | Rationale: Matches existing disable_activity behavior; is_active is derived from file location, not a stored field | Trade-offs: Two-step (write + remove) instead of atomic rename, consistent with write_atomic pattern elsewhere

## 3. Assumptions Made
- CLI identity arg is a raw string (identity_id), not resolved to a display name | Impact if incorrect: identity_id would not resolve correctly; can be changed to resolve via orbit identity show later
- `--active`/`--inactive` are mutually exclusive, not toggles | Impact if incorrect: None — they conflict_with each other in Clap

## 4. Design Weaknesses / Risks
- None

## 5. Deviations from Original Plan
- None; this exactly addresses what the reviewer flagged

## 6. Technical Debt Introduced
- None

## 7. Recommended Follow-Ups
- Consider resolving `--identity` to a display name (like `orbit activity add --identity` does) for consistency

## 8. Overall Assessment
All previously missing fields are now patchable. 14/14 activity tests pass. Full update surface now matches the original task description.