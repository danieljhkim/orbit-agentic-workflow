# Orbit-Owned Commit Flow via orbit-tools (V1) Plan

**Goal:** Let an agent propose a commit message and explicit file list, while Orbit performs the actual git commit through narrow reusable `orbit-tools` commands.
**Scope:** Agent response contract, runtime plumbing, git tool implementation in `orbit-tools`, and regression coverage for the simple `message + files[]` flow.
**Assumptions:** The workspace is a git repository when this flow is used. The first version can require exact file paths and can reject malformed or unsafe requests rather than trying to recover.
**Risks:** If validation is too loose, Orbit could commit unintended files. If the tool surface is too broad, we recreate a generic shell escape hatch instead of a safe reusable abstraction.

## Task 1: Design the commit-request contract

**Files:**
- Modify: `orbit-types/src/job.rs`
- Modify: `orbit-agent/src/providers/common.rs`
- Modify as needed: `orbit-core/src/command/job.rs`

**Steps:**
1. Define a minimal structured payload for agent responses that can carry `commit.message` and `commit.files`.
2. Decide where that payload lives inside the existing response/result contract so it stays backward-compatible for other activities.
3. Document the v1 constraints: explicit paths only, no partial hunks, no arbitrary git commands.

**Done When:**
- There is one clear response shape for commit intent.
- The contract is small enough for agents to produce reliably.

## Task 2: Add typed git commands in orbit-tools

**Files:**
- Modify or add: `orbit-tools/src/...`
- Modify any tool registration/plumbing files needed for built-in tools
- Modify as needed: `orbit-types/src/error.rs`

**Steps:**
1. Add narrow git commands such as staging explicit paths and creating a commit from a message.
2. Keep command inputs typed and minimal; do not add a generic git passthrough.
3. Validate repo scoping, path safety, and required inputs at the tool boundary.
4. Return clear errors for invalid requests or git failures.

**Done When:**
- `orbit-tools` exposes reusable git commands for stage/commit.
- The tool boundary enforces the core safety checks for v1.

## Task 3: Integrate Orbit runtime with the new git tools

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Modify supporting files as needed for tool invocation plumbing

**Steps:**
1. Parse the agent's commit request from the final response.
2. Validate that the message is non-empty and the file list is non-empty.
3. Invoke the typed `orbit-tools` git commands to stage only the requested files and create the commit.
4. Surface clear Orbit errors when the request is malformed or the tool execution fails.

**Done When:**
- Orbit can successfully create a commit from agent-supplied `message + files[]` using `orbit-tools`.
- Unsafe or malformed requests fail clearly without staging unrelated files.

## Task 4: Integrate with approval-oriented workflows

**Files:**
- Modify: `orbit-core/assets/skills/orbit-approve-task/SKILL.md`
- Modify any activity/skill docs that currently imply the agent must run `git commit` directly

**Steps:**
1. Update workflow docs so the agent proposes commit intent instead of performing the commit itself.
2. Align the approval/review workflow with the new Orbit-owned commit path.
3. Keep the workflow simple for v1 rather than expanding to broader git automation.

**Done When:**
- Orbit docs describe the actual supported commit pathway.
- The approval skill no longer depends on direct agent-side git mutation.

## Task 5: Add regression coverage

**Files:**
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Modify: `orbit-agent/tests/protocol_behavior.rs`
- Add focused tests in `orbit-tools` for the new git commands

**Steps:**
1. Add tests for parsing valid commit requests from agent responses.
2. Add tests that reject empty messages, empty file lists, and paths outside the repo.
3. Add a happy-path test that stages only the requested files and creates a commit.
4. Add a test confirming non-commit activities continue to work unchanged.

**Done When:**
- The new commit flow is covered end to end at a targeted level.
- Existing non-commit behavior remains stable.

## Final Verification
- `cargo test -p orbit-tools`
- `cargo test -p orbit-agent`
- `cargo test -p orbit-core job_runtime_behavior -- --nocapture`
- Manual: run an approval/review-oriented flow in a temp git repo and verify Orbit commits only the requested files with the requested message