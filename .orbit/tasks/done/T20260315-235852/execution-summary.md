## Status
success

## 1. Summary of Changes
Shifted `job_task_pipeline` to an isolated worktree strategy so task implementation no longer mutates the human checkout. `create_branch` now creates or reuses a dedicated `orbit/<task_id>` worktree, agent-invoked steps run with their current directory bound to the resolved task workspace, and the final checkout step now finalizes the retained worktree instead of switching the main repo branch.

## 2. Strategic Decisions
- Use dedicated helper scripts for worktree creation/finalization | Rationale: keeps the pipeline YAML small and makes the git worktree rules testable outside shell one-liners | Trade-offs: introduces two repo-local scripts to maintain.
- Bind agent cwd from the execution workspace contract | Rationale: fixes the root cause behind ambient checkout/worktree drift during `implement_change` and other agent steps | Trade-offs: agent steps now depend more explicitly on correct workspace propagation.
- Retain task worktrees instead of auto-deleting them in this pass | Rationale: avoids destructive cleanup while we are still validating the new execution model | Trade-offs: worktree cleanup remains a follow-up lifecycle concern.

## 3. Assumptions Made
- Reusing `orbit/<task_id>` and a stable task worktree path is preferable to creating a fresh ephemeral worktree each retry | Impact if incorrect: we may need a later retry/isolation policy update.
- Downstream steps should consume the propagated task worktree as `workspace_path` while preserving `repo_root` for ownership/finalization checks | Impact if incorrect: later PR or cleanup steps may need additional path fields.

## 4. Design Weaknesses / Risks
- Retained task worktrees will accumulate until cleanup policy is implemented | Severity: Medium | Mitigation: add explicit worktree lifecycle cleanup once the strategy is proven in production.
- Helper scripts emit JSON via shell `printf` and assume path/branch values do not contain JSON-breaking characters | Severity: Low | Mitigation: replace with a safer emitter or small Rust helper if path-shape requirements expand.

## 5. Deviations from Original Plan
- Repurposed `checkout_branch` into a worktree finalizer instead of deleting or renaming the step | Justification: preserves the existing job shape while removing the main-checkout branch mutation that caused the original friction.

## 6. Technical Debt Introduced
- Worktree cleanup is intentionally deferred | Recommended resolution: add explicit prune/archive rules tied to task/job lifecycle once the steady-state UX is validated.

## 7. Recommended Follow-Ups
- Wire PR/open-review flows to surface retained worktree metadata where it helps debugging and cleanup.
- Consider replacing shell JSON emission with a more robust helper if the worktree contract grows.

Validation:
- cargo test -p orbit-core --test job_runtime_behavior -- --nocapture
- cargo test -p orbit-core --test asset_formatting -- --nocapture
- cargo test --workspace