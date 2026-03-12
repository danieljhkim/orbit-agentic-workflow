# Approval-Aware Codex Invocation Plan

**Goal:** Let Orbit-driven Codex sessions complete legitimate repository mutations such as `git commit` by using an explicit approval/escalation path instead of failing with an opaque sandbox denial.
**Scope:** Codex provider invocation, any supporting runtime/config plumbing, regression tests, and workflow documentation. No change to the broader task lifecycle beyond making it executable.
**Assumptions:** Orbit should keep sandboxed execution as the default and only add a controlled escalation path. The right fix is in Orbit's provider/runtime layer rather than changing task instructions to avoid commits.
**Risks:** Overly broad defaults could weaken safety guarantees or make agent behavior surprising. The final design should keep the least-privilege default and require explicit approval/configuration for higher-risk execution.

## Task 1: Design the Codex approval/sandbox contract

**Files:**
- Modify: `orbit-agent/src/providers/codex/codex_cli.rs`
- Modify: `orbit-agent/src/providers/codex/codex_runtime.rs`
- Modify as needed: `orbit-core/src/config/raw.rs`
- Modify as needed: `orbit-core/src/config/runtime.rs`

**Steps:**
1. Decide whether Orbit should always pass a Codex approval policy (for example an approval-aware mode) or expose a provider/runtime setting that controls when it is used.
2. Keep `workspace-write` as the safe baseline unless an explicit configuration or workflow rule says otherwise.
3. Document the intended contract between Orbit and Codex for blocked commands.

**Done When:**
- Orbit has one clear, auditable place where Codex approval/sandbox behavior is defined.
- The design explains how commit-capable workflows avoid the current failure.

## Task 2: Implement provider/runtime support

**Files:**
- Modify: `orbit-agent/src/providers/codex/codex_cli.rs`
- Modify: `orbit-agent/src/providers/codex/codex_runtime.rs`
- Modify as needed: `orbit-core/src/config/raw.rs`
- Modify as needed: `orbit-core/src/config/runtime.rs`
- Modify as needed: `orbit-core/assets/config/default-config.toml`
- Modify as needed: `orbit-core/assets/config/default-config-repo.toml`

**Steps:**
1. Plumb any new approval or sandbox configuration through Orbit runtime/provider layers.
2. Update Codex argument construction so the agent can request escalation or run under the intended policy.
3. Preserve existing defaults for unrelated agent providers.

**Done When:**
- Codex invocation arguments reflect the new approval-aware behavior.
- The implementation is configurable or intentionally defaulted with rationale.

## Task 3: Add regression coverage

**Files:**
- Modify: `orbit-agent/tests/protocol_behavior.rs`
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Add or modify other focused tests as needed

**Steps:**
1. Add provider tests that assert the generated Codex arguments include the expected approval/sandbox settings.
2. Add runtime/config tests that verify defaults and configured overrides.
3. Preserve the existing workspace-write expectation where it still applies, adjusting assertions only where the contract intentionally changes.

**Done When:**
- Tests fail without the new behavior and pass with it.
- The new contract is covered at the provider and runtime boundaries.

## Task 4: Align workflow guidance and verify manually

**Files:**
- Modify: `orbit-core/assets/skills/orbit-approve-task/SKILL.md`
- Modify any related docs/config examples referenced by the workflow

**Steps:**
1. Update approval workflow guidance so it matches the actual Codex execution model.
2. Run targeted tests.
3. Manually exercise an approval or review flow that reaches a repository mutation and confirm Orbit now provides a workable path instead of failing with `Operation not permitted`.

**Done When:**
- Workflow docs no longer promise behavior that Orbit cannot execute.
- A manual repro path confirms the original failure is addressed.

## Final Verification
- `cargo test -p orbit-agent`
- `cargo test -p orbit-core codex_job_run_uses_workspace_write_sandbox -- --nocapture`
- Add/update targeted tests for any new config behavior
- Manual: reproduce the commit flow that previously failed and verify Orbit now supports the required approval/escalation behavior