# Identity Role Filter Implementation Plan

**Goal:** Add an optional role filter to orbit identity list without weakening layering or CLI determinism.
**Scope:** Include role-filtered list behavior for text and JSON output; exclude any identity mutation workflow or changes to identity show.
**Assumptions:** The current identity role enum remains the source of truth for accepted role values, and list ordering should remain deterministic after filtering.
**Risks:** If role parsing is duplicated in the CLI, the contract could drift from the catalog/runtime behavior; malformed role input must fail clearly rather than silently returning no results.

## Task 1: Add filtered identity listing below the CLI

**Files:**
- Modify: orbit-store/src/file/identity_store.rs
- Modify: orbit-core/src/runtime/mod.rs
- Modify: orbit-types/src/identity.rs if shared parsing helpers need reuse
- Test: unit coverage around filtered identity listing and invalid roles

**Steps:**
1. Add failing tests for listing identities by role and preserving deterministic ordering within filtered results.
2. Introduce a runtime or catalog API that accepts an optional role filter using the existing identity role contract.
3. Ensure invalid role values produce actionable validation errors instead of ambiguous empty output.
4. Re-run targeted lower-layer tests for identity filtering behavior.

**Done When:**
- Orbit exposes a deterministic filtered identity list API keyed by the canonical role vocabulary.

## Task 2: Wire the role filter into the CLI

**Files:**
- Modify: orbit-cli/src/command/identity.rs
- Test: orbit-cli/tests/identity_commands.rs

**Steps:**
1. Add an optional role flag to orbit identity list.
2. Route the requested role through the runtime API rather than implementing filtering only in the CLI formatter.
3. Verify both text and JSON list output only include identities matching the requested role.
4. Add regression coverage for valid filters, empty matches, and invalid role input.

**Done When:**
- orbit identity list supports role-based filtering consistently across text and JSON modes.

## Task 3: Update the CLI contract

**Files:**
- Modify: CLI_SPEC.md

**Steps:**
1. Document the new role filter option and any relevant output or validation guarantees.
2. Verify the documented behavior matches the implemented command semantics.

**Done When:**
- CLI documentation reflects the role-filtered identity list behavior.

## Final Verification
- cargo test -p orbit-store identity_store
- cargo test -p orbit-core
- cargo test -p orbit-cli identity_commands
- cargo test -p orbit-cli