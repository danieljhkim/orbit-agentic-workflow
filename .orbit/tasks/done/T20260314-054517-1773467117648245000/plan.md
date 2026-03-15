# orbit_home Naming Cleanup Plan

**Goal:** Make Orbit's root-related naming match the current semantics after the HOME fallback removal.
**Scope:** Runtime/context fields, config command output, and supporting tests/docs.
**Assumptions:** The selected Orbit root is now the canonical concept; HOME is no longer a first-class default.
**Risks:** Some users or scripts may already consume `config show --json` fields that mention `home`.

## Task 1: Rename internal runtime/context terminology

**Files:**
- Modify: `orbit-core/src/context.rs`
- Modify: `orbit-core/src/runtime/mod.rs`
- Modify: `orbit-core/src/runtime/builder.rs`
- Modify: any call sites that thread the old field name through the runtime

**Steps:**
1. Replace `orbit_home`-style field and method names with terminology that reflects the selected Orbit root.
2. Keep the codebase compiling cleanly across runtime/config callers.
3. Add/update targeted tests if the rename changes public APIs.

**Done When:**
- Internal runtime code no longer uses misleading HOME-specific names for the selected root.
- The resulting terminology is consistent across the runtime boundary.

## Task 2: Update config command output and docs

**Files:**
- Modify: `orbit-cli/src/command/config.rs`
- Modify: `orbit-cli/tests/config_commands.rs`
- Modify: relevant docs/help text if they mention the old HOME semantics

**Steps:**
1. Rename JSON fields and human-readable labels that currently say `home` or `ORBIT_HOME` when they actually mean the active root.
2. Decide whether to keep a compatibility alias temporarily for machine-readable output.
3. Update tests to assert the new output contract.

**Done When:**
- `config show` uses accurate terminology.
- Any compatibility story for existing consumers is explicit and tested.

## Task 3: Sweep remaining docs and compatibility references

**Files:**
- Modify: `CLAUDE.md`
- Modify: any runtime/config docs or tests that still refer to the old naming

**Steps:**
1. Remove stale HOME-specific naming from docs that describe the selected Orbit root.
2. Verify there are no accidental leftovers in non-test source.
3. Document any intentional compatibility aliases that remain.

**Done When:**
- User-facing language matches the new root model.
- Remaining compatibility names, if any, are deliberate rather than accidental.

## Final Verification
- `cargo test -p orbit-core`
- `cargo test -p orbit --test config_commands`
- `rg -n "orbit_home|ORBIT_HOME|home" orbit-core orbit-cli CLAUDE.md -g !target`