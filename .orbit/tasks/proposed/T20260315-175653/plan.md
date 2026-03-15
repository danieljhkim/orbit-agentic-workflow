# Job Step Model Selection Implementation Plan

**Goal:** Let each job step optionally declare an LLM model next to `agent_cli`, and have Orbit pass that choice through to supported agent CLIs.
**Scope:** Job schema/store/runtime/CLI/provider plumbing, bundled and active job YAML artifacts, and regression coverage for serialization plus invocation args.
**Assumptions:** Codex and Claude-compatible runtimes can accept a model argument, and Orbit can map the shared `model` field into provider-specific CLI flags where needed.
**Risks:** Provider CLIs may differ in model-flag support, partial threading could leave JSON/text output inconsistent, and bundled asset copies can drift if both source and live job artifacts are not updated together.

## Task 1: Extend the job step schema and persistence contract

**Files:**
- Modify: `orbit-types/src/job.rs`
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/file/job_store.rs`
- Modify: `orbit-core/src/command/job.rs`
- Modify: `orbit-cli/src/command/job.rs`

**Steps:**
1. Add an optional `model` field to `JobStep` and every raw/store contract used to read and write job definitions.
2. Keep add/update/show/list flows backward-compatible when `model` is omitted.
3. Update CLI JSON and human-readable output to surface the model when present.

**Done When:**
- Job definitions can round-trip with or without `steps[].model`.
- Existing jobs without a model continue to load unchanged.

## Task 2: Pass the model through agent execution

**Files:**
- Modify: `orbit-agent/src/agent/agent.rs`
- Modify: `orbit-agent/src/runtime/factory.rs`
- Modify: `orbit-agent/src/providers/codex/codex_runtime.rs`
- Modify: `orbit-agent/src/providers/codex/codex_cli.rs`
- Modify: `orbit-agent/src/providers/claude/claude_runtime.rs`
- Modify: `orbit-agent/src/providers/claude/claude_cli.rs`
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Extend the agent config/execution context so each job step can pass an optional model into provider runtimes.
2. Map the shared `model` field into the correct CLI args for Codex and Claude-compatible providers.
3. Preserve current behavior when `model` is not set.

**Done When:**
- A job step can request a model and the resulting agent invocation includes the expected provider-specific args.
- Jobs without `model` still invoke agents exactly as they do today.

## Task 3: Revise bundled job artifacts and add regression coverage

**Files:**
- Modify: `orbit-core/assets/jobs/job_review_tasks.yaml`
- Modify: `.orbit/jobs/jobs/job_review_tasks.yaml`
- Modify: `orbit-core/assets/jobs/job_task_pipeline.yaml`
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Modify: `orbit-cli/tests/job_commands.rs`
- Modify: `orbit-store/src/file/job_store.rs`

**Steps:**
1. Update the relevant job artifact examples to demonstrate optional `model` usage, starting with `job_review_tasks.yaml`.
2. Add store/runtime/CLI tests for YAML round-trip, JSON output, and provider arg construction with and without `model`.
3. Verify seeded assets and active copies stay aligned after the change.

**Done When:**
- The default job artifacts show how to specify `model` next to `agent_cli`.
- Tests cover both omitted and explicit model cases.

## Final Verification
- `cargo test -p orbit-store`
- `cargo test -p orbit-agent`
- `cargo test -p orbit-core job_runtime_behavior -- --nocapture`
- `cargo test -p orbit-cli job_commands -- --nocapture`