#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

fail=0

# Allowed internal `orbit-*` dependencies for each crate.
#
# Layout (post-split):
#   orbit-util  — leaf (no internal deps)
#   orbit-types — depends only on orbit-util
#   orbit-common — *transitional* shim; depends on orbit-types + orbit-util.
#                  Listed alongside orbit-types/orbit-util in caller allowlists
#                  for the migration window. To retire orbit-common, drop it
#                  from each caller's allowlist (and from this list) after
#                  imports have been migrated. See crates/orbit-common/RETIRE.md.
allowed_internal_deps() {
  case "$1" in
    orbit-util)
      echo ""
      ;;
    orbit-types)
      echo "orbit-util"
      ;;
    orbit-common)
      echo "orbit-types orbit-util"
      ;;
    orbit-policy | orbit-exec | orbit-knowledge | orbit-store | orbit-registry)
      echo "orbit-common orbit-types orbit-util"
      ;;
    orbit-tools)
      echo "orbit-common orbit-types orbit-util orbit-exec orbit-knowledge orbit-policy"
      ;;
    orbit-agent)
      echo "orbit-common orbit-types orbit-util orbit-tools"
      ;;
    orbit-engine)
      echo "orbit-agent orbit-common orbit-types orbit-util orbit-exec orbit-store orbit-tools"
      ;;
    orbit-core)
      echo "orbit-common orbit-types orbit-util orbit-engine orbit-policy orbit-store orbit-tools"
      ;;
    orbit-mcp)
      echo "orbit-common orbit-types orbit-util orbit-tools"
      ;;
    orbit-cli)
      echo "orbit-common orbit-types orbit-util orbit-core orbit-mcp"
      ;;
    *)
      echo ""
      ;;
  esac
}

contains_word() {
  local haystack="$1"
  local needle="$2"
  for word in $haystack; do
    if [[ "$word" == "$needle" ]]; then
      return 0
    fi
  done
  return 1
}

workspace_crates=(
  orbit-agent
  orbit-cli
  orbit-common
  orbit-core
  orbit-engine
  orbit-exec
  orbit-knowledge
  orbit-mcp
  orbit-policy
  orbit-registry
  orbit-store
  orbit-tools
  orbit-types
  orbit-util
)

for crate in "${workspace_crates[@]}"; do
  manifest="$repo_root/crates/${crate}/Cargo.toml"
  if [[ ! -f "$manifest" ]]; then
    echo "missing manifest for ${crate}: ${manifest}"
    fail=1
    continue
  fi

  allowed="$(allowed_internal_deps "$crate")"
  while IFS= read -r dep; do
    if [[ -n "$dep" ]] && ! contains_word "$allowed" "$dep"; then
      echo "forbidden dependency '${dep}' found in ${manifest}"
      echo "  allowed internal deps for ${crate}: ${allowed:-<none>}"
      fail=1
    fi
  done < <(
    rg -o "^[[:space:]]*orbit-[a-z-]+[[:space:]]*=" "$manifest" |
      sed -E 's/^[[:space:]]*(orbit-[a-z-]+)[[:space:]]*=.*/\1/'
  )
done

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo "dependency direction guard passed"
