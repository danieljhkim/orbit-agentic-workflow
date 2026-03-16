#!/bin/sh
set -eu

repo_root="${1:?repo_root is required}"
workspace_path="${2:?workspace_path is required}"

cleanup_strategy="retained"

if [ "$workspace_path" = "$repo_root" ]; then
  cleanup_strategy="main_checkout_unchanged"
elif ! git -C "$workspace_path" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "workspace_path is not a git worktree: $workspace_path" >&2
  exit 1
fi

printf '{"exit_code":0,"workspace_path":"%s","repo_root":"%s","cleanup_strategy":"%s"}' \
  "$workspace_path" "$repo_root" "$cleanup_strategy" > "$ORBIT_OUTPUT_FILE"
