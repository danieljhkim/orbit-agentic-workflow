# Claude Skill Symlink Implementation Plan

**Goal:** Ensure `orbit init` creates per-skill symlinks in `.claude/skills` in addition to `.agents/skills`.
**Scope:** Update init target resolution and link creation logic, add regression coverage for the new Claude link root, and document behavior if CLI-visible behavior changes. Do not change the seeded skill contents or broader init layout beyond the new symlink root.
**Assumptions:** Claude should consume the same skill directory structure as `.agents`, and the link targets should continue to point at the initialized `.orbit/skills/<skill-id>` directories.
**Risks:** Duplicating link-management logic could create drift between `.agents` and `.claude`; migration or repair behavior may need to handle pre-existing non-directory or broken paths safely.

## Task 1: Refactor init link targeting

**Files:**
- Modify: `orbit-core/src/command/init.rs`
- Review: `orbit-core/src/fs_utils.rs`

**Steps:**
1. Identify the current init target model for skill link roots and refactor it so multiple link destinations can be managed without duplicating behavior.
2. Reuse the existing per-skill symlink creation and repair logic for both `.agents/skills` and `.claude/skills`.
3. Preserve current force/idempotence semantics and precise error handling for invalid pre-existing paths.

**Done When:**
- `orbit init` manages both `.agents/skills` and `.claude/skills` through one coherent init path.
- Existing `.agents` behavior remains unchanged while `.claude` receives matching links.

## Task 2: Add regression coverage

**Files:**
- Modify: `orbit-cli/tests/init_commands.rs`

**Steps:**
1. Extend the existing init tests to assert that `.claude/skills` is created and populated with per-skill symlinks.
2. Add or update tests for idempotent reruns and for migration/repair of broken or legacy Claude skill link paths where relevant.
3. Keep assertions deterministic across supported platforms.

**Done When:**
- Tests fail without the Claude link behavior and pass once it is implemented.
- Coverage demonstrates both `.agents` and `.claude` link roots are initialized correctly.

## Task 3: Verify shipped behavior

**Files:**
- Review: `orbit-cli/src/command/init.rs`
- Review: `CLI_SPEC.md`
- Review: `ARCHITECTURE.md`

**Steps:**
1. Confirm whether the new symlink root changes documented init behavior.
2. Update documentation only if the user-facing or architectural contract now needs to mention `.claude/skills`.
3. Run focused verification for init behavior after implementation.

**Done When:**
- Documentation matches shipped behavior.
- Verification covers the updated init path.

## Final Verification
- `cargo test -p orbit-cli init_commands -- --nocapture`
- `cargo test -p orbit-core`
- `cargo test --workspace`