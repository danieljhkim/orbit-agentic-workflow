#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cli_src_dir="$repo_root/crates/orbit-cli/src"

forbidden=(
  "orbit_agent"
  "orbit_engine"
  "orbit_exec"
  "orbit_policy"
  "orbit_store"
  "orbit_tools"
)

# Match `<crate>::` references in orbit-cli sources, ignoring occurrences
# inside `"..."` string literals (e.g. tracing target labels like
# `"orbit_engine::activity_job::cli_runner"`) and `//` line comments. Block
# comments (`/* ... */`) are not handled — none exist in the current tree.
scan_crate() {
  local crate="$1"
  local file
  while IFS= read -r -d '' file; do
    local matches
    matches="$(sed -E 's://.*$::; s/"[^"]*"/""/g' "$file" \
      | grep -nE "\\b${crate}::" || true)"
    if [[ -n "$matches" ]]; then
      while IFS= read -r line; do
        echo "${file#"$repo_root/"}:${line}"
      done <<< "$matches"
    fi
  done < <(find "$cli_src_dir" -type f -name '*.rs' -print0)
}

failed=0
for crate in "${forbidden[@]}"; do
  matches="$(scan_crate "$crate")"
  if [[ -n "$matches" ]]; then
    echo "forbidden import reference found in orbit-cli: ${crate}::"
    echo "$matches"
    failed=1
  fi
done

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi

echo "orbit-cli import guard passed"
