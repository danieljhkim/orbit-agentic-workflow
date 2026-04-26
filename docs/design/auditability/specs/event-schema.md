# Spec: Audit Event Schema

Audit events in Orbit are split by operational level, but every channel must preserve enough stable identity, timing, status, and join keys for incident reconstruction.

## Why This Exists

Orbit has more than one audit storage mechanism. This spec prevents "audit event" from becoming an ambiguous term and defines the minimum contract each channel must keep.

## Channel Contracts

### Command Audit Event

Command audit events are SQLite rows represented by `AuditEvent`.

Required invariants:

- `execution_id` is non-empty and unique within the command audit table.
- `timestamp`, `command`, `role`, `status`, `exit_code`, `duration_ms`, `working_directory`, and `pid` are always present.
- `status` is one of `success`, `failure`, or `denied`.
- A denied command uses `status: denied` and a non-zero exit code.
- Error messages are redacted before insertion.
- Command rows do not embed full provider request/response bodies.

Failure modes:

- If the command exits through an early return, the RAII guard still writes one row.
- If audit insertion fails during `Drop`, the CLI prints a warning and does not mask the command result.
- If runtime initialization fails before the guard exists, no command audit row is written.

### V2 Activity/Job Audit Event

Activity/job audit events are JSONL entries represented by `V2AuditEvent`.

Required invariants:

- `schemaVersion`, `event_type`, `event_id`, `ts`, `run_id`, and `agent_identity` are always present.
- `event_id` is unique within a `run_id`.
- Child events set `parent_event_id` when emitted under a parent context.
- `workspace_path` remains optional; CLI/library callers with a real workspace should attach it, while smoke and stub hosts may omit it.
- Body variants use the `body_kind` discriminator.
- A run starts with a run-start event and finishes with a run-finished event whenever execution reaches the dispatch wrapper.

Failure modes:

- If JSONL persistence fails, in-memory event capture remains the load-bearing smoke-verification path.
- If a worker thread emits events, it must install the parent stack snapshot before emission.
- If an activity/job run fails before writer construction, no v2 JSONL is expected.

### Loop Audit Event

Loop audit events are JSONL entries represented by `LoopAuditEvent`.

Required invariants:

- Every event includes `run_id`.
- Session-scoped events include `session_id`.
- HTTP request/response bodies and tool input/output payloads are stored as blob hashes, not inline bodies.
- Policy denials record the denied tool and reason.
- Blob bytes are redacted before hashing and writing.

Failure modes:

- If a blob write fails, the sink may return an `error:<message>` reference; readers must treat that as a failed payload capture.
- If no durable sink is installed, `NullSink` intentionally drops events and blobs. Production activity/job callers should not use it.

### Invocation Trace

Invocation traces are metric records, not transcript records.

Required invariants:

- Records identify `job_run_id`, `activity_id`, agent, optional model, task ids, duration, token usage, and tool-call summaries.
- Invocation traces must not be treated as proof that transcript-level audit exists.
- A missing invocation trace is a metrics gap; a missing audit event is an audit gap.

## Migration Rules

- New audit event body variants must be added to the relevant enum and documented here or in a linked feature spec.
- Existing body variants are append-only unless an ADR records a breaking migration.
- If a channel moves storage roots, readers should either support the previous root explicitly or document that historical files are manual-only.

## Agent Signature

Last revised by codex / gpt-5 for [T20260426-0605].
