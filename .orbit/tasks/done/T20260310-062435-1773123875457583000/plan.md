# Non-Destructive Init Reinitialization Plan

**Goal:** Make `orbit init` always refresh default identities and skills without wiping the rest of the Orbit workspace.
**Scope:** Update init behavior, the default seeding helpers, CLI output expectations, and init regression tests.
**Assumptions:** Only the shipped default identity and skill files should be refreshed automatically; user-created tasks, jobs, activities, config, and other artifacts must remain intact during plain `orbit init`.
**Risks:** Overwriting too broadly could destroy user customizations in `.orbit/identities` or `.orbit/skills`; the implementation needs a clear policy for default-managed files versus user-owned files.

## Task 1: Define non-destructive refresh semantics

**Files:**
- Modify: `orbit-core/src/command/init.rs`
- Modify: `orbit-core/src/command/identity.rs`
- Modify: `orbit-core/src/command/skill.rs`
- Modify: `orbit-cli/src/command/init.rs`

**Steps:**
1. Decide which files are managed defaults that should always be rewritten during `orbit init`.
2. Update identity and skill seeding helpers so default-managed files are refreshed in place instead of only being created when missing.
3. Keep `--force` as the only path that removes unrelated Orbit artifacts or resets the entire root.
4. Adjust init output if needed so it clearly reports refreshed versus newly created defaults.

**Done When:**
- Running `orbit init` refreshes the shipped identity and skill defaults without deleting unrelated workspace state.

## Task 2: Repair linked skill roots during normal init

**Files:**
- Modify: `orbit-core/src/command/init.rs`
- Test: `orbit-cli/tests/init_commands.rs`

**Steps:**
1. Verify the current repair logic for `.agents/skills` and `.claude/skills` also works correctly during non-force refresh.
2. Add coverage for stale or outdated default skill content under `.orbit/skills` plus stale linked skill entries under agent-facing roots.
3. Confirm normal init restores the expected linked layout without needing manual directory deletion.

**Done When:**
- A user can restore default Orbit skills by rerunning `orbit init` without deleting `.orbit/skills` or linked agent skill directories first.

## Task 3: Protect tasks, jobs, and other runtime artifacts

**Files:**
- Test: `orbit-cli/tests/init_commands.rs`
- Modify if needed: `orbit-core/src/command/init.rs`

**Steps:**
1. Add regression coverage proving plain `orbit init` preserves existing task and job artifacts while refreshing identities and skills.
2. Keep the existing `--force` reset behavior for full destructive reinitialization.
3. Document any remaining limitations around user-edited default identity or skill files if the refresh policy intentionally overwrites them.

**Done When:**
- Plain init is safe for routine identities/skills re-seeding, while `--force` remains the opt-in full reset path.

## Final Verification
- `cargo test -p orbit --test init_commands -- --nocapture`
- targeted verification that default identity and skill files are refreshed after local modification without removing existing tasks/jobs