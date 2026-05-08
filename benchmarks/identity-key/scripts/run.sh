#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BENCH_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${BENCH_DIR}/../.." && pwd)"
OUT_DIR="${BENCH_DIR}/v1/runs"

cd "${REPO_ROOT}"
cargo run -q -p orbit-knowledge --example identity_key_benchmark -- --output "${OUT_DIR}"
