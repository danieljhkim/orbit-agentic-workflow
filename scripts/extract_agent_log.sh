#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 /absolute/path/to/log.jsonl" >&2
  exit 2
fi

LOG_PATH="$1"

if [[ "${LOG_PATH:0:1}" != "/" ]]; then
  echo "Error: path must be absolute: ${LOG_PATH}" >&2
  exit 2
fi

if [[ ! -f "$LOG_PATH" ]]; then
  echo "Error: file not found: ${LOG_PATH}" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is required. Install with: brew install jq" >&2
  exit 127
fi

jq -cs '
  [ .[]
    | select(.type == "event_msg")
    | select(.payload.type == "agent_reasoning")
    | {
        timestamp: .timestamp,
        type: .payload.type,
        title: (
          (.payload.text // "")
          | capture("^\\*\\*(?<t>[^*]+)\\*\\*"; "m")?.t
          // null
        ),
        text: (.payload.text // "")
      }
  ]
' "$LOG_PATH"