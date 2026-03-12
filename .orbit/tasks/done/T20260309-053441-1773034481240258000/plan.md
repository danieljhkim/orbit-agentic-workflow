# Task Comment CLI Support Implementation Plan

**Goal:** Add deterministic comment-appending support to task mutation commands while staying aligned with Orbit's current file-backed task architecture.
**Scope:** Extend the shared task contract, runtime mutation API, CLI flags, and file-backed task persistence so `--comment` appends task comments on add, update, approve, and reject.
**Assumptions:** Task comments should be first-class task data exposed through the shared `Task` model; comment writes should happen inside the existing runtime mutation boundary; the current implementation target is the file-backed task store used by the runtime today.
**Risks:** `comments` already exists in the task bundle document but is not exposed through the shared `Task` contract or CLI output; comment author attribution for `task add` and `task update` is not yet explicit in the current command surface and must be defined carefully.

## Task 1: Align the shared task contract with file-backed comments

**Files:**
- Modify: `orbit-types/src/task.rs`
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/file/task_store.rs`

**Steps:**
1. Add a shared task comment type and expose task comments on the domain `Task`.
2. Extend task create and update store contracts so callers can append comments without replacing existing entries.
3. Update the file task store to round-trip comments through task bundles and preserve append order.
4. Add store-level regression coverage proving appended comments keep prior history intact.

**Done When:**
- Task comments are part of the shared task model.
- The file-backed task store appends comments safely and deterministically.

## Task 2: Wire runtime and CLI support

**Files:**
- Modify: `orbit-core/src/command/task.rs`
- Modify: `orbit-cli/src/command/task.rs`
- Modify: `orbit-cli/tests/task_commands.rs`

**Steps:**
1. Extend `TaskAddParams`, `TaskUpdateParams`, `approve_task`, and `reject_task` inputs to accept an optional comment append request.
2. Define deterministic comment author rules for each command, reusing existing explicit identities where they already exist.
3. Add `--comment` flags to `task add`, `task update`, `task approve`, and `task reject`.
4. Ensure each command appends comments within the existing mutation flow and still emits the correct `OrbitEvent` and audit record.
5. Expose comments in `task show --json` and human-readable output as needed for verification.

**Done When:**
- The four CLI commands accept `--comment` and append rather than replace.
- Runtime behavior remains auditable and follows existing lifecycle semantics.

## Task 3: Add domain-level regression coverage

**Files:**
- Modify: `orbit-core/src/lib.rs`
- Modify: `orbit-cli/tests/task_commands.rs`
- Modify: relevant file-task-store test modules under `orbit-store/src/file/`

**Steps:**
1. Add CLI tests for add, update, approve, and reject with `--comment`.
2. Assert appended comments preserve prior entries and record the expected author and message payload.
3. Cover error paths such as empty comment values if the CLI or runtime rejects them.
4. Run targeted task-related test suites after implementation.

**Done When:**
- New behavior is covered by deterministic regression tests across CLI, runtime, and file-backed storage.

## Final Verification
- `cargo test -p orbit-cli task_`
- `cargo test -p orbit-core task_`
- `cargo test -p orbit-store file::task_store`