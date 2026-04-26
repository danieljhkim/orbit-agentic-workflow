# Groundhog — Implementation Status

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-26 (public deviation verb removed, [T20260426-0603])

This file is the living drift ledger for Groundhog. `2_design.md` explains the current implementation in prose; this file answers the narrower question: what is left to implement, clean up, or make explicit?

---

## 1. How To Read This

- **Shipped** means the capability exists and the docs currently match the code closely enough.
- **Partial** means there is working code, but the shape still differs from the intended v1 contract.
- **Open** means the docs describe a target that the shipped code does not yet realize.
- **Decision** means implementation should wait for an explicit architecture choice.

---

## 2. Status Table

| Area | Status | Current reality | Next step | Owner | Last checked |
|------|--------|-----------------|-----------|-------|--------------|
| Dedicated Groundhog activity | Shipped | `ActivityV2Spec::Groundhog` and the dedicated runner are live. | Keep aligned with any future activity-schema changes. | codex | 2026-04-22 |
| Structured checkpoint plans | Shipped | `TaskPlan` checkpoints with typed `success_criteria` and per-checkpoint budgets are live. | Keep plan quality expectations explicit in docs. | codex | 2026-04-22 |
| Scratch-branch rewind | Shipped | `WorkspaceSnapshot` creates `groundhog/<task_id>/day-<n>`, rewinds on failure, and squash-merges on success. | Preserve this as the baseline contract unless approval-safe refs replace direct task-branch commits. | codex | 2026-04-22 |
| Prompt-facing memory vs audit split | Partial | Runtime persists `artifacts.chronicle` plus `groundhog/state.json`, not a clean `GroundhogMemory` and `GroundhogRun` split. | Decide migration shape, then rewrite persistence around separate prompt and audit records. | codex | 2026-04-22 |
| Legacy deviation surface removal | Partial | `orbit.groundhog.checkpoint_deviate` is no longer registered in the public tool surface; `checkpoint_deviate`, `deviation_stack`, and `DayOutcome::DeviatedTo` internals still exist even though v1 does not use them. | Remove the remaining enum variants and serializer baggage or explicitly defer them to a vnext design. | codex | 2026-04-26 |
| Groundhog using shared verifier | Partial | The shared verifier exists, but the runner still uses an inline local verifier helper. | Rewire the runner onto `checkpoint_verifier.rs` so verifier semantics stop drifting. | codex | 2026-04-22 |
| Verifier run persistence | Open | Attempt records do not store `VerifierRun` data on pass or fail. | Persist verifier runs into the audit-side Groundhog record once the shared verifier is adopted. | codex | 2026-04-22 |
| Tool-call audit fidelity | Partial | `Attempt.tool_calls` exists in the type but the runner writes empty vectors today. | Capture per-attempt Groundhog tool-call summaries or remove the field until it is real. | codex | 2026-04-22 |
| Scratch branch and committed ref audit fields | Open | Attempt and checkpoint records omit explicit `scratch_branch` and `committed_ref` fields promised by the intended v1 model. | Add those fields when the persistence split is redesigned. | codex | 2026-04-22 |
| Side-effect memory in later prompts | Partial | Successful days persist `side_effects`, but later prompts replay summaries only. | Decide the minimal safe side-effect summary format and inject it into prompt memory. | codex | 2026-04-22 |
| `attempt_budget_default` semantics | Partial | Activity-level default currently acts as a floor because checkpoints are parsed with their own default and the runner uses `max(...)`. | Make it a true fallback or rename/document the floor semantics explicitly in code and asset schema. | codex | 2026-04-22 |
| Approval-safe checkpoint materialization | Decision | Success commits land directly on the task branch today. | Decide whether Groundhog needs internal refs before approval, then either implement them or codify direct commits as the permanent contract. | codex | 2026-04-22 |
| Provider plumbing | Partial | Dispatcher gates Groundhog to wired HTTP providers, but the runner still fetches `api_key_for(\"anthropic\")` directly. | Make provider handling explicit in runner plumbing or constrain the type surface further. | codex | 2026-04-22 |
| Observability and debug surface | Open | No dedicated Groundhog chronicle/debug view or Groundhog-specific metrics surface exists yet. | Add only the metrics that answer real operational questions: attempts per checkpoint, rewind counts, verifier pass/fail counts, blocked outcomes. | codex | 2026-04-22 |

---

## 3. Highest-Priority Work

If Groundhog work resumes soon, the most leverage is in this order:

1. Remove deviation-era leftovers so the shipped tool and type surface matches the stated v1 contract.
2. Move the runner onto the shared verifier and persist verifier runs.
3. Redesign persistence into a cleaner prompt-memory versus audit split.
4. Decide whether direct task-branch checkpoint commits are acceptable long term.
5. Add the smallest useful observability surface after the execution model itself is trustworthy.

---

## 4. Notes

- This file is intentionally narrower than [2_design.md](./2_design.md). It is the queue, not the full spec.
- If a row stays `Decision` or `Open` for long, add or update an ADR in [4_decisions.md](./4_decisions.md) so the ambiguity is named explicitly.
