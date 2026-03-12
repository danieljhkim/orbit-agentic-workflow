# Rejected Status Implementation Plan

**Goal:** Add a dedicated `rejected` task status and use it consistently for proposal and review rejections.
**Scope:** Update the task domain model, runtime transition rules, CLI contracts, persistence formats, and the Orbit skills/docs that describe approval and rejection flows.
**Assumptions:** `archived` remains a separate administrative/archive state rather than the canonical destination for rejected work.
**Risks:** Lifecycle changes can ripple through task storage paths, audit records, built-in activities, and skill instructions; partial updates could leave the system describing one workflow while enforcing another.

## Task 1: Define the lifecycle and status contract

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-cli/src/command/task.rs`
- Modify: `orbit-core/src/command/task.rs`
- Modify: `README.md`

**Steps:**
1. Add `rejected` to the canonical task status set and decide the allowed transitions to and from that state.
2. Update CLI help, status parsing, and user-facing lifecycle descriptions to include `rejected`.
3. Adjust runtime rejection behavior so `proposed -> rejected` and `review -> rejected` are the canonical outcomes.
4. Clarify how `archive` and `unarchive` should behave now that rejection is no longer modeled as archival.

**Done When:**
- The lifecycle contract clearly distinguishes `rejected` from `archived` and the CLI/runtime enforce the new rejection path.

## Task 2: Update persistence, auditability, and tests

**Files:**
- Modify: `orbit-store/src/file/task_store.rs`
- Modify: `orbit-store/src/sqlite/migration.rs`
- Modify: `orbit-types/src/event.rs`
- Modify: `orbit-store/src/sqlite/audit_store.rs`
- Test: `orbit-cli/tests/task_commands.rs`
- Test: `orbit-core/src/lib.rs`
- Test: `orbit-store` task storage tests relevant to status directories or persisted status values

**Steps:**
1. Ensure file-backed and SQLite-backed task persistence can store and retrieve `rejected`.
2. Update any status-directory or serialization logic that currently assumes rejected tasks are archived or backlogged.
3. Confirm audit/event output still reflects proposal and review rejection cleanly under the new status model.
4. Add regression coverage for rejecting proposed and review tasks, plus any changed archive/unarchive behavior.

**Done When:**
- Persisted tasks, runtime events, and CLI tests all recognize `rejected` without breaking existing task workflows.

## Task 3: Update Orbit skills and built-in guidance

**Files:**
- Modify: `.orbit/skills/orbit-skills/SKILL.md`
- Modify: `.orbit/skills/orbit-manage-tasks/SKILL.md`
- Modify: `.orbit/skills/orbit-approve-task/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-skills/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-manage-tasks/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-approve-task/SKILL.md`
- Modify: built-in activities or other docs that mention rejection moving to `archived` or `backlog`

**Steps:**
1. Find every skill or activity that documents proposal rejection or review rejection.
2. Update lifecycle diagrams, command examples, and response contracts to reference `rejected`.
3. Remove conflicting guidance that still tells agents rejected work becomes `archived` or returns to `backlog`.

**Done When:**
- Agents following shipped skills will consistently use and expect the `rejected` state.

## Final Verification
- `cargo test -p orbit-types -- --nocapture`
- `cargo test -p orbit-core -- --nocapture`
- `cargo test -p orbit --test task_commands -- --nocapture`