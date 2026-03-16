# Remove Direct Agent Run Command Plan

**Goal:** Eliminate the legacy standalone `orbit agent run` command and the supporting runtime path that only exists for that CLI surface.
**Scope:** CLI command wiring, legacy core command helpers, tests, and any stale docs/help text tied to the direct agent-run flow.
**Assumptions:** The engine/executor-based activity flow is now the intended agent execution path, and the direct CLI run path is legacy.
**Risks:** Some session-read helpers may still be reused elsewhere, so deletion should be based on the actual call graph rather than broad assumptions.

## Task 1: Remove the direct CLI surface

**Files:**
- Modify: `orbit-cli/src/command/agent.rs`
- Modify: `orbit-cli/src/command/mod.rs`
- Modify: any CLI help or command registration surfaces that still expose `agent run`

**Steps:**
1. Remove the `orbit agent run` subcommand and any related argument structs or execution glue.
2. Update the top-level CLI command registration so the legacy command is no longer advertised.
3. Adjust user-facing help output expectations if tests assert on command listings.

**Done When:**
- The CLI no longer exposes `orbit agent run`.

## Task 2: Delete the legacy core runtime path

**Files:**
- Modify/Delete: `orbit-core/src/command/agent.rs`
- Modify: `orbit-core/src/command/mod.rs`
- Modify: any runtime/session helper modules that still reference the removed path

**Steps:**
1. Remove `run_agent_task` / `run_agent_task_with_options` and any helper logic that only exists for the direct CLI run flow.
2. Keep only the session APIs that are still genuinely used elsewhere; delete the rest.
3. Confirm the engine/executor path remains intact and does not regress.

**Done When:**
- No dead legacy direct-agent-run logic remains in the core command layer.

## Task 3: Clean up tests and stale references

**Files:**
- Modify/Delete: `orbit-core/tests/agent_run_behavior.rs`
- Modify/Delete: `orbit-cli/tests/agent_commands.rs`
- Modify: any docs, skills, or examples that still mention the removed command

**Steps:**
1. Remove or replace tests that only cover the deleted direct command path.
2. Update any help-text, docs, or examples that still mention `orbit agent run`.
3. Run focused validation plus a broader workspace test pass.

**Done When:**
- Tests and docs align with the new supported agent execution model.

## Final Verification
- `cargo test -p orbit-cli`
- `cargo test -p orbit-core`
- `cargo test --workspace`