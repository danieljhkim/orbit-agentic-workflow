# Adopt isolated worktree execution for task implementation

## Goal
Run task implementation in a dedicated git worktree so agent changes do not churn the human's active checkout.

## Scope
- task/job pipeline execution model
- branch/worktree creation and reuse rules
- agent tool cwd/repo binding
- cleanup and lifecycle for task worktrees
- bundled activities/jobs that currently assume in-place branch switching

## Work items
1. Define the worktree strategy: naming, location, branch mapping, reuse, and cleanup policy.
2. Update `job_task_pipeline` and related activities so branch creation happens in a dedicated worktree instead of the main checkout.
3. Ensure `implement_change`, tests, and git tools execute inside the task worktree, not ambient agent cwd.
4. Decide how PR/open-review steps discover the correct repo/worktree context after implementation.
5. Add regression coverage proving the main checkout remains unchanged while agent work happens in an isolated worktree.

## Done when
- task implementation runs in a dedicated worktree
- the human's active checkout is not used as the agent's working directory
- branch/worktree lifecycle is explicit and tested
- bundled pipeline assets reflect the new model