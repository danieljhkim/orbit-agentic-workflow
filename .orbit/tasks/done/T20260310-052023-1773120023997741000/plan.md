# Task Status Naming Clarity Plan

**Goal:** Prevent repeated `in_progress` versus `in-progress` mistakes when updating Orbit task status.
**Scope:** Improve task-status UX and guidance in the CLI, docs, and skills that agents follow.
**Assumptions:** Persisted task data may continue using snake_case internally even if the CLI accepts kebab-case or additional aliases.
**Risks:** Partial updates could leave agents seeing conflicting status formats across help text, docs, and generated skill content.

## Task 1: Audit and align status terminology

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-cli/src/command/task.rs`
- Modify: `README.md`
- Modify: `.orbit/skills/orbit-skills/SKILL.md`
- Modify: `.orbit/skills/orbit-manage-tasks/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-skills/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-manage-tasks/SKILL.md`

**Steps:**
1. Trace where `in_progress` and `in-progress` are exposed to users or agents.
2. Choose a single clear UX policy for CLI-facing status values and document the internal versus external representation if both remain.
3. Update help text, lifecycle examples, and skill instructions to remove ambiguity.

**Done When:**
- A reader can tell which status value to type into `orbit task update --status ...` without guessing.

## Task 2: Implement mitigation in the CLI path

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-cli/src/command/task.rs`
- Test: `orbit-cli/tests/task_commands.rs`

**Steps:**
1. Implement the selected mitigation for the common `in_progress` input path.
2. Ensure command output or parsing behavior nudges users toward the correct status spelling.
3. Add or update tests for successful parsing and for any improved error messaging or help behavior.

**Done When:**
- The known failure case is either accepted directly or produces a much clearer recovery path.

## Final Verification
- `cargo test -p orbit-cli task_commands -- --nocapture`
- `cargo test -p orbit-types -- --nocapture`