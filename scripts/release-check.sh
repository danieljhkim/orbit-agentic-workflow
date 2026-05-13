#!/usr/bin/env bash
# Verify the release-version invariant for the /plugin install orbit chain.
#
# The npm package version, the plugin manifest version, and the latest
# GitHub Release tag must all agree. Drift here means `npx -y
# @orbit-tools/cli@latest mcp serve` downloads a binary tagged at a
# different release than the plugin manifest pins.
#
# Exits 0 when every source agrees, 1 on drift, 2 on missing prerequisites.
# Documented entry point: `make release-check`.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

NPM_PKG="@orbit-tools/cli"
PLUGIN_MANIFEST="plugin/.claude-plugin/plugin.json"
NPM_PACKAGE_JSON="plugin/npm/package.json"

require_bin() {
  local bin="$1"
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "release-check: required binary '$bin' not on PATH" >&2
    exit 2
  fi
}

require_bin jq

for f in "$PLUGIN_MANIFEST" "$NPM_PACKAGE_JSON"; do
  if [[ ! -f "$f" ]]; then
    echo "release-check: $f not found (run from repo root)" >&2
    exit 2
  fi
done

plugin_version="$(jq -r .version "$PLUGIN_MANIFEST")"
npm_package_version="$(jq -r .version "$NPM_PACKAGE_JSON")"

if [[ -z "$plugin_version" || "$plugin_version" == "null" ]]; then
  echo "release-check: $PLUGIN_MANIFEST has no .version field" >&2
  exit 2
fi
if [[ -z "$npm_package_version" || "$npm_package_version" == "null" ]]; then
  echo "release-check: $NPM_PACKAGE_JSON has no .version field" >&2
  exit 2
fi

npm_registry_version=""
if command -v npm >/dev/null 2>&1; then
  if v="$(npm view "$NPM_PKG" version 2>/dev/null)"; then
    npm_registry_version="$v"
  else
    echo "release-check: npm view $NPM_PKG version failed (registry unreachable?)" >&2
  fi
else
  echo "release-check: npm not on PATH; skipping registry check" >&2
fi

gh_tag_version=""
if command -v gh >/dev/null 2>&1; then
  if tag="$(gh release list -L 1 --json tagName -q '.[0].tagName' 2>/dev/null)"; then
    gh_tag_version="${tag#v}"
  else
    echo "release-check: gh release list failed (not authenticated or no releases?)" >&2
  fi
else
  echo "release-check: gh not on PATH; skipping GitHub Release check" >&2
fi

printf '%-32s %s\n' "$PLUGIN_MANIFEST"           "$plugin_version"
printf '%-32s %s\n' "$NPM_PACKAGE_JSON"          "$npm_package_version"
printf '%-32s %s\n' "npm view $NPM_PKG"          "${npm_registry_version:-<skipped>}"
printf '%-32s %s\n' "gh release list -L 1"       "${gh_tag_version:-<skipped>}"

drift=0
if [[ "$plugin_version" != "$npm_package_version" ]]; then
  echo "DRIFT: $PLUGIN_MANIFEST ($plugin_version) != $NPM_PACKAGE_JSON ($npm_package_version)" >&2
  drift=1
fi
if [[ -n "$npm_registry_version" && "$plugin_version" != "$npm_registry_version" ]]; then
  echo "DRIFT: $PLUGIN_MANIFEST ($plugin_version) != npm view $NPM_PKG ($npm_registry_version)" >&2
  drift=1
fi
if [[ -n "$gh_tag_version" && "$plugin_version" != "$gh_tag_version" ]]; then
  echo "DRIFT: $PLUGIN_MANIFEST ($plugin_version) != latest gh release tag ($gh_tag_version)" >&2
  drift=1
fi

if [[ "$drift" -ne 0 ]]; then
  cat >&2 <<EOF

release-check failed. See docs/RELEASE.md for the procedure.
The /plugin install orbit chain assumes all four sources agree.
EOF
  exit 1
fi

if [[ -z "$npm_registry_version" || -z "$gh_tag_version" ]]; then
  echo "release-check: local sources agree on $plugin_version; remote checks were skipped." >&2
  exit 0
fi

echo "release-check: all sources agree on $plugin_version"
