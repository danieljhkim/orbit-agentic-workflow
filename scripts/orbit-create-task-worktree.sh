#!/bin/sh
set -eu

repo_root="${1:?repo_root is required}"
task_id="${2:?task_id is required}"
base_ref="${3:?base_ref is required}"

branch="orbit/$task_id"
repo_name="$(basename "$repo_root")"

if [ "${ORBIT_WORKTREE_ROOT:-}" != "" ]; then
  worktree_root="$ORBIT_WORKTREE_ROOT/$repo_name"
else
  worktree_root="$(cd "$repo_root/.." && pwd)/../worktrees/$repo_name"
fi

worktree_path="$worktree_root/$task_id"
mkdir -p "$worktree_root"

if [ -e "$worktree_path" ]; then
  if git -C "$worktree_path" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    current_branch="$(git -C "$worktree_path" rev-parse --abbrev-ref HEAD)"
    if [ "$current_branch" != "$branch" ]; then
      echo "existing worktree at '$worktree_path' is on branch '$current_branch', expected '$branch'" >&2
      exit 1
    fi
    printf '{"exit_code":0,"workspace_path":"%s","repo_root":"%s","branch":"%s"}' \
      "$worktree_path" "$repo_root" "$branch" > "$ORBIT_OUTPUT_FILE"
    exit 0
  fi

  echo "worktree path exists but is not a git worktree: $worktree_path" >&2
  exit 1
fi

start_point="$base_ref"
if ! git -C "$repo_root" rev-parse --verify "$start_point^{commit}" >/dev/null 2>&1; then
  start_point="origin/$base_ref"
fi
git -C "$repo_root" rev-parse --verify "$start_point^{commit}" >/dev/null 2>&1

if git -C "$repo_root" show-ref --verify --quiet "refs/heads/$branch"; then
  git -C "$repo_root" worktree add "$worktree_path" "$branch"
else
  git -C "$repo_root" worktree add -b "$branch" "$worktree_path" "$start_point"
fi

printf '{"exit_code":0,"workspace_path":"%s","repo_root":"%s","branch":"%s"}' \
  "$worktree_path" "$repo_root" "$branch" > "$ORBIT_OUTPUT_FILE"
