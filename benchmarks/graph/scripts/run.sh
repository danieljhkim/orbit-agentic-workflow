#!/usr/bin/env bash
# Single-cell benchmark driver. Thin wrapper around run.py.
#
# Usage:
#   benchmarks/graph/scripts/run.sh <arm> <task_id> <seed> [--provider claude|codex] [--no-probe]
#
# Environment (optional; normally set by sweep.py or the Makefile):
#   GRAPH_VERSION      — which version's tasks/runs to target (default: v3)
#   SWEEP_ID           — groups runs into a sweep
#   RUN_ORDER_INDEX    — 0-based index within a shuffled sweep order
#   NONCE              — cold-cache nonce (uuid4 if unset)
#   CLAUDE_BIN         — override claude binary path
#   CODEX_BIN          — override codex binary path

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/run.py" "$@"
