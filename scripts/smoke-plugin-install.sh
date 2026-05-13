#!/usr/bin/env bash
# End-to-end smoke for the /plugin install orbit chain.
#
# Runs against the *published* @orbit-tools/cli@latest from npm (not the
# local working tree) so it catches version drift between the npm proxy
# and the GitHub Release binary it fetches.
#
# Steps:
#   1. npx -y @orbit-tools/cli@latest --version
#        -> exercises postinstall: tarball download + sha256 verification
#   2. drive `orbit mcp serve` over stdio with a JSON-RPC handshake
#      (initialize + tools/list) and assert at least one `orbit.*` tool
#      appears in the response.
#
# Pass: exit 0. Fail: non-zero with the relevant stderr captured.
# Supported: macOS arm64 / x86_64, Linux x86_64 / arm64. Not Windows.
set -euo pipefail

require_bin() {
  local bin="$1"
  if ! command -v "$bin" >/dev/null 2>&1; then
    echo "smoke-plugin-install: required binary '$bin' not on PATH" >&2
    exit 2
  fi
}

require_bin node
require_bin npx
require_bin npm

case "$(uname -s)" in
  Darwin|Linux) ;;
  *)
    echo "smoke-plugin-install: unsupported OS '$(uname -s)' — supported: Darwin, Linux" >&2
    exit 2
    ;;
esac

NPM_PKG="@orbit-tools/cli"
TMPDIR_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_ROOT"' EXIT

# Cache npx installs inside the temp dir so we exercise a clean download.
export npm_config_cache="$TMPDIR_ROOT/npm-cache"
mkdir -p "$npm_config_cache"

echo "--- step 1: npx -y $NPM_PKG@latest --version ---"
VERSION_OUT="$TMPDIR_ROOT/version.txt"
if ! npx -y "$NPM_PKG@latest" --version >"$VERSION_OUT" 2>"$TMPDIR_ROOT/version.err"; then
  echo "FAIL: npx -y $NPM_PKG@latest --version exited non-zero" >&2
  echo "--- stderr ---" >&2
  cat "$TMPDIR_ROOT/version.err" >&2
  exit 1
fi
binary_version="$(tr -d '[:space:]' <"$VERSION_OUT" || true)"
if [[ -z "$binary_version" ]]; then
  echo "FAIL: orbit --version produced no output" >&2
  cat "$TMPDIR_ROOT/version.err" >&2
  exit 1
fi
echo "orbit --version => $binary_version"

# Also confirm the npm-registry version, for the smoke report.
npm_version="$(npm view "$NPM_PKG" version 2>/dev/null || echo '<unknown>')"
echo "$NPM_PKG@latest on npm => $npm_version"

echo "--- step 2: MCP handshake over stdio ---"
RPC_IN="$TMPDIR_ROOT/rpc-in.txt"
RPC_OUT="$TMPDIR_ROOT/rpc-out.txt"
RPC_ERR="$TMPDIR_ROOT/rpc-err.txt"

cat >"$RPC_IN" <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke-plugin-install","version":"0.0.1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
EOF

# Feed the requests, then keep stdin open briefly so the server has time
# to flush its responses before EOF closes the channel.
{
  cat "$RPC_IN"
  sleep 5
} | timeout 120 npx -y "$NPM_PKG@latest" mcp serve >"$RPC_OUT" 2>"$RPC_ERR" || rc=$?
rc="${rc:-0}"

# `timeout` returns 124 when it kills the server; that's expected because
# we never send `shutdown`. Anything else from the binary side is a fail.
if [[ "$rc" -ne 0 && "$rc" -ne 124 && "$rc" -ne 143 ]]; then
  echo "FAIL: mcp serve exited with $rc" >&2
  echo "--- stderr ---" >&2
  cat "$RPC_ERR" >&2
  echo "--- stdout ---" >&2
  cat "$RPC_OUT" >&2
  exit 1
fi

if ! grep -q '"jsonrpc":"2.0"' "$RPC_OUT"; then
  echo "FAIL: no JSON-RPC frames in mcp serve stdout" >&2
  echo "--- stdout ---" >&2
  cat "$RPC_OUT" >&2
  echo "--- stderr ---" >&2
  cat "$RPC_ERR" >&2
  exit 1
fi

# tools/list response should advertise at least one orbit_* tool.
# Tool names are emitted with underscores on the MCP wire (orbit-mcp's
# sanitize_tool_name replaces `.` with `_` for client compatibility), so the
# canonical `orbit.task.show` selector arrives here as `orbit_task_show`.
if ! grep -q '"orbit_' "$RPC_OUT"; then
  echo "FAIL: tools/list response contained no orbit_* tools" >&2
  echo "--- stdout ---" >&2
  cat "$RPC_OUT" >&2
  echo "--- stderr ---" >&2
  cat "$RPC_ERR" >&2
  exit 1
fi

orbit_tool_count="$(grep -o '"orbit_[a-z_]*"' "$RPC_OUT" | sort -u | wc -l | tr -d '[:space:]')"
echo "tools/list returned $orbit_tool_count distinct orbit_* tools"

echo "PASS: /plugin install orbit chain serves MCP successfully (orbit $binary_version)"
