## Context
Comments, history entries, and review messages are append-heavy. Keeping them as arrays in `task.yaml` causes whole-file rewrites and bad merge behavior for the exact fields most likely to be touched by parallel agents.

## Decision
Store lifecycle and history events in `events.jsonl`, task comments in `comments.jsonl`, and review threads under `review-threads/`. Each append gets a stable event, comment, or message ID and preserves actor and timestamp metadata.

## Consequences
- Concurrent append operations can merge by ID rather than by YAML text position.
- Audit readers can stream events without parsing the envelope.
- Review prose can be stored as Markdown while thread metadata stays structured.
- Cost: reads that need the complete task now load several files. Event-log corruption handling and partial-write recovery become part of the store contract.