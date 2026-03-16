1. Add an Orbit-owned internal activity execution path for workflow automation and use it for worktree creation, commit creation, and PR opening.
2. Refactor `job_task_pipeline` so `implement_change` only implements/tests and a new `commit_changes` step handles repository finalization.
3. Replace bash-script worktree orchestration with Orbit-native worktree creation/reuse and structured step outputs.
4. Convert `open_pr` into Orbit-owned automation that derives title/body from task metadata, especially `title` and `execution_summary`.
5. Add regression coverage for worktree lifecycle, commit ownership, dirty-tree handling, and PR payload generation.