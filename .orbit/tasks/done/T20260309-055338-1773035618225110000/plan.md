# Identity Inspection Commands Implementation Plan

**Goal:** Add deterministic, read-only identity discovery commands to the Orbit CLI.
**Scope:** Include top-level identity subcommands for listing identities and showing one identity; exclude any identity mutation workflow.
**Assumptions:** Identities continue to be sourced from the configured identity root, and the existing YAML identity format remains the source of truth.
**Risks:** Listing may require new catalog/runtime APIs; output shape should be explicit so we do not create an unstable or ambiguous CLI contract.

## Task 1: Add runtime and catalog read APIs

**Files:**
- Modify: orbit-store/src/file/identity_store.rs
- Modify: orbit-core/src/runtime/mod.rs
- Modify: orbit-core/src/lib.rs or other identity-export wiring if needed
- Test: targeted unit coverage near the catalog/runtime identity access points

**Steps:**
1. Add failing tests for listing and showing identities through the lowest appropriate layer.
2. Extend the identity catalog/runtime with explicit read APIs for listing identities and resolving a single identity.
3. Ensure ordering is deterministic and malformed files surface actionable errors.
4. Re-run targeted tests for the new catalog/runtime behavior.

**Done When:**
- Orbit exposes a deterministic read path for all configured identities plus the existing single-identity lookup behavior.

## Task 2: Add CLI identity commands

**Files:**
- Modify: orbit-cli/src/command/mod.rs
- Create: orbit-cli/src/command/identity.rs
- Modify: orbit-cli/src/main.rs if command wiring requires it
- Test: orbit-cli/tests/* identity command coverage

**Steps:**
1. Add orbit identity to the top-level clap command surface.
2. Implement list and show <identity_id> as presentation-only wrappers over the runtime APIs.
3. Make text output readable and deterministic; keep error messages precise for unknown identities.
4. Add regression tests that cover seeded identities and show output for a specific id.

**Done When:**
- orbit identity list and orbit identity show <identity_id> both work end-to-end through the CLI.

## Task 3: Document the command contract

**Files:**
- Modify: CLI_SPEC.md
- Update additional docs only if the visible CLI contract changes elsewhere

**Steps:**
1. Document the new top-level command surface and any relevant output or behavior guarantees.
2. Verify the documented contract matches the shipped behavior.

**Done When:**
- CLI documentation reflects the new identity inspection commands.

## Final Verification
- cargo test -p orbit-store
- cargo test -p orbit-core
- cargo test -p orbit-cli identity
- cargo test -p orbit-cli