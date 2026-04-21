# Spec: Workspace Snapshot

Groundhog attempts execute on a git-backed scratch branch. The snapshot contract is simple: create a clean, inspectable branch-local workspace for one attempt; on failure, rewind the task branch back to its original commit; on success, squash the scratch branch back into one checkpoint commit. This spec describes that contract as implemented today.

## Why This Exists

Groundhog's retry loop only makes sense if the next attempt starts from a clean tracked workspace state. Asking the agent to "undo what you just tried" is not reliable enough. The runtime needs a deterministic rewind mechanism that is visible in git and safe across process restarts.

## Invariants

- Snapshot creation requires a named task branch. Detached HEAD is an error.
- Snapshot creation requires a clean tracked workspace. Groundhog does not snapshot over staged or unstaged tracked edits.
- Each attempt uses a scratch branch named `groundhog/<task_id>/day-<n>`.
- Pre-existing untracked files are preserved and must not be absorbed into the scratch-branch capture.
- Rewind and success-commit both fail closed if the task branch head has moved away from `snapshot_ref` during the attempt.

## Snapshot Creation

When `WorkspaceSnapshot::create(...)` succeeds, it has already:

1. Resolved and canonicalized the workspace path.
2. Read the current task branch name.
3. Verified that tracked files are clean.
4. Recorded `snapshot_ref` as the task branch's HEAD commit.
5. Recorded the set of pre-existing untracked paths to preserve.
6. Created and checked out the scratch branch.

The task branch remains the authoritative branch for the task. The scratch branch is attempt-local working state, not a user-facing workflow branch.

## Rewind Contract

`WorkspaceSnapshot::rewind(...)` performs the failure path:

1. If the workspace is still on the scratch branch, capture the scratch state as a commit on that branch.
2. Check out the task branch.
3. Verify that the task branch still points at `snapshot_ref`.
4. `git reset --hard <snapshot_ref>` on the task branch.

The scratch branch is retained after rewind for inspection. Rewind only guarantees cleanup for git-tracked workspace state. Non-git side effects remain out of scope.

## Success Contract

`WorkspaceSnapshot::commit_success(...)` performs the success path:

1. Require the workspace to still be on the scratch branch.
2. Capture the scratch state as a commit on that branch.
3. Check out the task branch.
4. Verify that the task branch still points at `snapshot_ref`.
5. Reset the task branch back to `snapshot_ref`.
6. `git merge --squash` the scratch branch onto the task branch.
7. Create one commit whose message is derived from the checkpoint summary.
8. Delete the scratch branch.

If the scratch branch has no tracked changes relative to `snapshot_ref`, success short-circuits after deleting the scratch branch and creates no new commit.

## Failure Modes

- If the task branch moved during the attempt, Groundhog aborts instead of clobbering newer commits.
- If a scratch-branch name already exists, snapshot creation aborts and names the colliding branch.
- If the success summary is blank, `commit_success(...)` rejects the call.
- If the workspace is on an unexpected branch during rewind or success, Groundhog aborts instead of guessing.

## Agent Signature

Last revised by `codex` on 2026-04-21.
