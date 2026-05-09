#!/usr/bin/env bash

set -euo pipefail

# Ensure we're in the repository root
cd "$(git rev-parse --show-toplevel)"

echo "Pruning unreachable worktrees..."
git worktree prune

echo "Removing worktrees..."
main_path=$(pwd)

git worktree list | while read -r line; do
    # Extract path and the branch name (last field in brackets)
    wt_path=$(echo "$line" | awk '{print $1}')
    branch_field=$(echo "$line" | awk '{print $NF}')
    
    # Strip brackets if present
    branch="${branch_field#\[}"
    branch="${branch%\]}"
    
    # Skip the main working tree
    if [[ "$wt_path" == "$main_path" ]]; then
        continue
    fi
    
    # Skip if branch is main or agent-main
    if [[ "$branch" == "main" || "$branch" == "agent-main" ]]; then
        continue
    fi
    
    echo "Removing worktree: $wt_path (branch: $branch)"
    git worktree remove --force "$wt_path" || true
done

echo "Removing branches..."
git for-each-ref --format '%(refname:short)' refs/heads/ | while read -r branch; do
    if [[ "$branch" != "main" && "$branch" != "agent-main" ]]; then
        echo "Deleting branch: $branch"
        git branch -D "$branch" || true
    fi
done

echo "Cleanup complete."
