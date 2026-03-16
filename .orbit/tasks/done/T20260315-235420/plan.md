# Task Attribution Simplification Plan

**Goal:** Remove redundant task decision/identity fields and let Orbit derive task actors from execution context while storing proposal/review decisions in history.
**Scope:** Task model/storage, task command runtime behavior, tool/actor provenance plumbing, CLI/docs output, and regression tests.
**Assumptions:** `orbit tool run ...` task operations should be attributable to agents; non-tool CLI task operations should be attributable to humans; activity-driven executions should use the activity artifact's `identity_id` when available.
**Risks:** Existing task data and downstream JSON consumers may rely on fields we remove, and the new history surface must stay easy to inspect.

## Task 1: Define the simplified task attribution contract

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-types/src/event.rs`
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/file/task_store.rs`

**Steps:**
1. Remove the redundant task decision fields from the persisted task model.
2. Define the history/audit shape that will carry proposal/review decisions, actors, and notes instead.
3. Decide which task-level attribution fields remain part of the task record versus becoming derived/runtime concerns.
4. Document any compatibility behavior needed for legacy task bundles or JSON consumers.

**Done When:**
- The task schema no longer treats proposal/review decisions as duplicated top-level metadata.
- Orbit has one clear source of truth for task decision provenance.

## Task 2: Implement automatic actor inference for task operations

**Files:**
- Modify: `orbit-core/src/command/task.rs`
- Modify: `orbit-core/src/command/tool.rs`
- Modify any directly affected runtime/context code that determines the effective actor
- Read/Modify: activity execution paths that already carry `identity_id` if needed

**Steps:**
1. Introduce a clear runtime rule for attributing task operations as agent-driven versus human-driven.
2. Use existing execution context to fill task actor/history data automatically instead of trusting manual CLI identity strings.
3. Ensure activity-based executions use the known `identity_id` from the activity artifact when that path is the source of the task mutation.
4. Preserve meaningful approval/rejection notes while moving them into history rather than top-level task fields.

**Done When:**
- Task add/update/approve/reject flows no longer require redundant manual actor inputs.
- Orbit can attribute task mutations consistently from runtime context.

## Task 3: Remove redundant task CLI identity arguments and update UX

**Files:**
- Modify: `orbit-cli/src/command/task.rs`
- Modify: `orbit-cli/tests/task_commands.rs`
- Modify: `orbit-core/assets/skills/orbit/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-create-task/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-approve-task/SKILL.md`
- Modify mirrored `.orbit` skill copies if they remain part of the active workflow surface

**Steps:**
1. Remove task CLI flags that duplicate runtime-known actor identity, including approval/rejection actor inputs and routine manual creator/assignee inputs.
2. Update `task show` / JSON output so the resulting history remains understandable after the top-level decision fields are removed.
3. Add coverage for agent vs human attribution, activity-identity attribution, and legacy-task compatibility behavior.
4. Update skill/workflow guidance so future agents stop passing identity fields Orbit now infers.

**Done When:**
- The task CLI is simpler and no longer asks operators for identity data Orbit already knows.
- Docs/tests describe and verify the new attribution behavior.

## Final Verification
- `cargo test -p orbit-core`
- `cargo test -p orbit-cli --test task_commands`
- `cargo test --workspace`