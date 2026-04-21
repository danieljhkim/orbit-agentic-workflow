# Groundhog V1

**Status:** Draft  
**Owner:** TBD  
**Last updated:** 2026-04-19

> *"Retry the checkpoint, not the whole task. Keep the lesson, discard the mess."*

Groundhog v1 is a structured execution mode for Orbit `backend: http` agents. A
Groundhog run executes a pre-authored list of checkpoints one at a time. Each
attempt starts with:

- a fresh HTTP agent session
- a clean git-backed workspace snapshot
- a small, stable prompt context composed from the task plan and prior
  successful checkpoint summaries

Failed attempts do not carry their full scratch history forward. Successful
checkpoints do.

This document is intentionally narrow. It defines the smallest version worth
shipping to validate the model.

## 1. Goals

Groundhog v1 exists to prove four things:

1. Checkpoint-scoped execution is more reliable than an unstructured
   `agent_loop` for implementation tasks with clear subgoals.
2. Resetting both agent context and git-tracked workspace state on failure
   materially reduces retry thrash.
3. Runtime verification of mechanical success criteria catches false-positive
   "success" claims without bloating agent context.
4. A small success-only memory is enough for later checkpoints to build on
   earlier work.

## 2. Non-Goals

The following are explicitly out of scope for v1:

- `backend: cli` support
- arbitrary executor-authored deviation stacks
- critic-on-retry or multi-agent retry mediation
- chronicle compaction
- cross-task chronicle reuse
- automatic rollback of non-git side effects
- provider-specific cache behavior as a semantic requirement

If Groundhog needs one of these to be useful, v1 has not been scoped tightly
enough.

## 3. Applicability

Groundhog v1 is only valid when all of the following are true:

- the activity uses `backend: http`
- the task plan already contains structured checkpoints
- the workspace is on a named task branch
- the workspace has no tracked changes before a checkpoint attempt starts
- the checkpoint's expected work is primarily git-tracked file mutation

Groundhog v1 should not be the default for research, open-ended debugging, or
tasks whose primary outputs live outside the workspace.

## 4. Execution Model

### 4.1 Checkpoints

Groundhog consumes structured checkpoints from the task plan.

Each checkpoint has:

- `id`
- `spec`
- `success_criteria`
- `attempt_budget` with default `3`

Checkpoints execute sequentially. v1 does not support executor-authored
sub-checkpoints or stack-based deviation.

### 4.2 Attempt Lifecycle

For each checkpoint:

1. Runtime snapshots the task branch into a scratch branch.
2. Runtime starts a fresh HTTP agent session.
3. Agent receives:
   - system/tools/skills context
   - task plan
   - summaries of prior successful checkpoints
   - current checkpoint spec
   - last failure report for this checkpoint, if retrying
4. Agent works within the attempt using normal tool calls.
5. Agent must terminate the attempt by emitting one of:
   - `orbit.groundhog.checkpoint_success`
   - `orbit.groundhog.checkpoint_failure`
6. Runtime either verifies and commits success, or rewinds and retries/fails.

### 4.3 Outcomes

Each checkpoint ends in one of three runtime outcomes:

- `success`: verifier passes, workspace result is kept, summary is recorded
- `abandoned`: attempt budget exhausted without success
- `blocked`: runtime stops early because policy or environment makes retry
  inappropriate

`blocked` is a runtime state, not a separate agent verb in v1.

## 5. Prompt and Memory Model

Groundhog v1 separates prompt-facing memory from durable audit data.

### 5.1 Prompt-Facing Memory

Only the following survive into later prompts:

- checkpoint summaries from prior successful checkpoints
- persisted side-effect summaries from prior successful checkpoints
- the most recent failure report when retrying the same checkpoint

This is the only Groundhog memory loaded back into the model.

### 5.2 Audit-Only Data

The following are persisted for review/debugging but never reloaded into prompt
context automatically:

- full tool-call transcripts for an attempt
- verifier command output
- scratch-branch refs
- timing and retry metadata

This split is deliberate. v1 should validate the execution model without
coupling correctness to ever-growing prompt history.

### 5.3 Caching

If the provider supports prompt caching, runtime should place cache breakpoints
only at stable boundaries:

- system + tools + skills
- task plan
- success-only checkpoint memory

Scratch context and failed attempt transcripts are not part of the stable cache
prefix. Groundhog remains valid even if caching is unavailable.

## 6. Workspace Rewind Model

Groundhog v1 uses git-backed scratch branches.

### 6.1 Snapshot

At attempt start:

- record task-branch `HEAD` as `snapshot_ref`
- create scratch branch `groundhog/<task_id>/day-<n>`
- perform all attempt-local edits on the scratch branch

### 6.2 Failure

On `checkpoint_failure`:

- capture the attempt's final scratch state to the scratch branch
- return to the task branch
- `git reset --hard <snapshot_ref>`
- persist the failure report and retry metadata

The scratch branch may be retained for inspection during the run.

### 6.3 Success

On `checkpoint_success` followed by passing verifier checks:

- capture final scratch changes
- return to the task branch
- reset task branch to `snapshot_ref`
- squash-merge the scratch branch onto the task branch
- create one checkpoint commit with message derived from the summary
- delete the scratch branch

The v1 invariant is that successful checkpoint state is represented durably in
git. If Orbit policy does not allow immediate user-visible commits on the task
branch before approval, the runtime may store the checkpoint commit on an
internal Groundhog-managed ref and materialize it onto the task branch later.

### 6.4 Side Effects

Groundhog v1 only guarantees rewind for git-tracked workspace state.

Non-git side effects are not automatically reverted. In Groundhog mode, tools
that can perform irreversible actions should be disabled by default unless the
activity or policy explicitly opts into them.

## 7. Agent Protocol

Groundhog v1 keeps the protocol surface small:

| Tool | Payload | Meaning |
|------|---------|---------|
| `execute` | normal tool calls | progress within the current attempt |
| `orbit.groundhog.checkpoint_success` | `{summary, side_effects}` | propose that the checkpoint is complete |
| `orbit.groundhog.checkpoint_failure` | `{what_tried, what_happened, next_attempt_plan}` | close the attempt as failed |

v1 intentionally does not include `checkpoint_deviate`. If the plan is wrong,
the checkpoint should fail and the runtime should abandon/block rather than
letting the executor rewrite the plan mid-run.

## 8. Verification

Groundhog v1 distinguishes between two classes of success criteria.

| Kind | Example | Evaluated by |
|------|---------|--------------|
| Mechanical | `command`, `file_exists`, `file_contains` | runtime verifier |
| Semantic | "error message is user-friendly" | executor judgment |

### 8.1 Verification Flow

When the agent emits `checkpoint_success`:

1. Runtime loads the checkpoint's mechanical criteria.
2. Runtime evaluates them out-of-band from the model loop.
3. If any mechanical criterion fails:
   - runtime synthesizes a failure report
   - runtime rewinds the workspace
   - the attempt counts as failed
4. If all mechanical criteria pass:
   - the checkpoint closes as success
   - the summary is added to prompt-facing memory

Passing verifier output is not added to the agent transcript on success.

### 8.2 Criterion Schema

Groundhog v1 uses typed success criteria.

```yaml
checkpoints:
  - id: ckpt_01
    spec: "Update `foo` to handle null inputs."
    success_criteria:
      - kind: command
        command: "make build"
        expect_exit: 0
      - kind: file_exists
        path: "crates/foo/src/lib.rs"
      - kind: file_contains
        path: "crates/foo/src/lib.rs"
        pattern: "handle_null"
      - kind: semantic
        statement: "Behavior matches the requested null-input handling."
    attempt_budget: 3
```

String-only criteria are not part of v1.

## 9. Data Model

Groundhog v1 keeps two separate persisted views.

### 9.1 Prompt-Facing Memory

Persisted as task artifact data loaded into later prompts.

```rust
pub struct GroundhogMemory {
    pub task_id: OrbitId,
    pub plan_id: OrbitId,
    pub completed: Vec<CompletedCheckpoint>,
}

pub struct CompletedCheckpoint {
    pub checkpoint_id: String,
    pub summary: String,
    pub side_effects: Vec<SideEffect>,
    pub committed_ref: String,
    pub completed_at: Timestamp,
}
```

### 9.2 Audit Record

Persisted for review/debugging, but not reloaded automatically into prompt
context.

```rust
pub struct GroundhogRun {
    pub task_id: OrbitId,
    pub plan_id: OrbitId,
    pub checkpoints: Vec<CheckpointRun>,
}

pub struct CheckpointRun {
    pub checkpoint_id: String,
    pub outcome: CheckpointOutcome,
    pub attempts: Vec<AttemptRecord>,
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
}

pub enum CheckpointOutcome {
    Success,
    Abandoned { reason: String },
    Blocked { reason: String },
}

pub struct AttemptRecord {
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
    pub failure_report: Option<FailureReport>,
    pub workspace_reverted: bool,
    pub scratch_branch: String,
    pub verifier_runs: Vec<VerifierRun>,
    pub tool_calls: Vec<ToolCallRecord>,
}
```

The key invariant is simple:

- prompt-facing memory contains only successful checkpoint summaries
- retry history lives only in the audit record

## 10. Orbit Integration

### 10.1 Activity Shape

Groundhog v1 should be its own activity kind rather than an overloaded
`agent_loop` flag.

```yaml
kind: groundhog
backend: http
provider: claude
wall_clock_timeout_seconds: 3600
plan_source: "{{ input.task_id }}"
attempt_budget_default: 3
```

### 10.2 Plan Requirement

Groundhog v1 requires a pre-existing structured checkpoint plan. Plan generation
is out of scope for this document. A planner activity may exist, but Groundhog
execution begins only after the plan is already present.

### 10.3 Builtins

Groundhog v1 requires:

- `orbit.groundhog.checkpoint_success`
- `orbit.groundhog.checkpoint_failure`
- `orbit.groundhog.side_effect`

Useful but non-essential follow-up tooling:

- `orbit.groundhog.chronicle` debug/read-only view

### 10.4 Persistence

Persist on every attempt close:

- Groundhog audit record
- latest failure report for the active checkpoint
- scratch branch ref

Persist on every checkpoint success:

- prompt-facing memory entry
- resulting git ref for the successful checkpoint state

## 11. Observability

Groundhog v1 should emit at least:

- checkpoints attempted
- checkpoints succeeded
- checkpoints abandoned
- attempts per checkpoint
- workspace rewinds
- verifier pass/fail counts

The first question v1 should answer is not "is the design elegant?" It is "does
this reduce retry thrash and false-positive success compared with ordinary
`agent_loop` execution?"

## 12. Limitations

Groundhog v1 is intentionally conservative.

- If the checkpoint plan is wrong, v1 can only fail fast; it cannot repair the
  plan mid-run.
- If the task requires irreversible side effects, v1 offers no rollback
  guarantee.
- If semantic success depends on subtle judgment, the executor can still be
  wrong.
- If the task is exploratory, forcing it into checkpoints will likely hurt more
  than help.

These are acceptable tradeoffs for v1. The purpose of v1 is to validate the
checkpoint + rewind + verifier + success-only-memory loop, not to solve every
autonomy problem up front.

## 13. Follow-Up Work

If v1 works, the most likely follow-ons are:

1. planner-generated checkpoint plans
2. controlled deviation support
3. critic-on-retry
4. chronicle compaction
5. richer debugging and scoreboard surfaces

Those should be informed by v1 data rather than assumed in advance.
