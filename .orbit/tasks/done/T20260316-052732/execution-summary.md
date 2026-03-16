# Execution Summary - Fix store divergence in commit/PR automations
Agent Name: claude
Agent Model: claude-sonnet-4-6

## Status
success

## Orbit Task
Task ID: T20260316-052732

## 1. Summary of Changes
Removed the `execution_summary` store dependency from both `commit_task_changes` and `open_pr_from_task` automations. `commit_task_changes` now reads `summary` from `input["summary"]` (passed forward from `implement_change`'s output schema) and appends it as the git commit message body. `open_pr_from_task` now builds the PR body from `input["commit_message"]` + `input["changed_files"]` (passed forward from `commit_task_changes`'s output) and writes that body to `execution_summary` on the task when transitioning to review, satisfying the lifecycle guard. `TaskAutomationUpdate` gained an `execution_summary` field wired through `apply_task_automation_update`. Activity schemas for `commit_changes` and `open_pr` updated in both `.orbit/activities/active/` and `orbit-core/assets/activities/`. Regression test `commit_task_changes_uses_summary_from_input` added.

## 2. Strategic Decisions
- Use pipeline input fields instead of task store reads | Rationale: eliminates the worktree/main-store divergence class of failure; automations now have everything they need in their inputs | Trade-offs: agents must include `summary` in their output schema — but `implement_change` already required it.
- `open_pr_from_task` writes body as execution_summary via TaskAutomationUpdate | Rationale: the lifecycle guard (in-progress -> review requires non-empty execution_summary) is a separate concern from the PR body source; writing the body to the store satisfies it without a separate store write by the agent | Trade-offs: execution_summary in the store is now the PR body, not the agent's detailed summary — this was already the case in the original design.

## 3. Assumptions Made
- `implement_change`'s output schema `summary` field is always populated on success | Impact if incorrect: commit step fails with a clear error at the guard.
- `commit_task_changes`'s `commit_message` and `changed_files` are always passed forward to `open_pr_from_task` via the pipeline | Impact if incorrect: PR body will be empty-ish but the automation will not fail.

## 4. Design Weaknesses / Risks
- execution_summary in the store is now the PR body (commit message + changed files), not a rich agent summary | Severity: Low | Mitigation: acceptable given the original design also conflated them.

## 5. Deviations from Original Plan
- Added `execution_summary` field to `TaskAutomationUpdate` and `apply_task_automation_update` to satisfy lifecycle guard | Justification: discovered the in-progress -> review guard during implementation; this was not anticipated in the plan but is the cleanest fix.

## 6. Technical Debt Introduced
None

## 7. Recommended Follow-Ups
- Consider decoupling execution_summary (agent's detailed technical summary) from PR body — they serve different purposes.

## 8. Overall Assessment
Clean fix. Eliminates a fragile store-read in both automation functions and makes the pipeline self-contained through its own step outputs.