#!/usr/bin/env bash
set -euo pipefail

INTERVAL=1800
TOTAL=8000
START=$(date +%s)
END=$((START + TOTAL))
NEXT=$START

while [ "$NEXT" -lt "$END" ]; do
  NOW=$(date +%s)

  if [ "$NOW" -lt "$NEXT" ]; then
    sleep $((NEXT - NOW))
  fi

  echo "[orbit] run at $(date -Is)"

  if ! orbit job run job_task_pipeline; then
    echo "[orbit] job failed at $(date -Is)" >&2
    # optional: break or continue depending on semantics
  fi

  NEXT=$((NEXT + INTERVAL))
done