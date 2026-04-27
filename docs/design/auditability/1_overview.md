# Auditability — Overview

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-26

Auditability is Orbit's answer to the operator question that matters after an agent touches a real repository: what happened, why did it happen, and who is accountable? It is not one log file. It is a set of structured audit channels that cover CLI commands, Orbit tool mutations, activity/job runs, provider turns, tool calls, filesystem denials, task attribution, and metrics side effects. [2_design.md](./2_design.md) describes the current implementation; [3_vision.md](./3_vision.md) captures the gaps between the current audit channels and Orbit's longer-term audit promise.

---

## 1. Motivation

Orbit runs fleets of agents against user-owned repositories. That makes auditability a product feature rather than an observability afterthought:

1. **Users need replayable accountability.** README and positioning docs promise that every meaningful action has enough context to answer what, why, and who. The dedicated design folder added in [T20260426-0605] makes that promise easier to review as implementation grows.
2. **Audit data has multiple levels of detail.** A CLI command audit row answers "which command ran?" A v2 activity/job envelope answers "which workflow step ran?" A loop audit event and blob reference answer "which provider payload or tool call happened?" Those layers must stay related without being collapsed into one oversized record.
3. **Agent identity has to survive every boundary.** Task author fields, command audit roles, v2 `agent_identity`, invocation metrics, and commit/task metadata all need to point back to a concrete actor or model.
4. **Secret handling is part of the audit contract.** Orbit aims to preserve faithful payloads, but audit storage must redact provider keys and sensitive environment-derived values before writing durable artifacts.
5. **Coverage gaps must be explicit.** Silent paths are bugs. Where the implementation is still incomplete, those gaps belong in a coverage matrix rather than tribal memory.

---

## 2. Core Concepts

### 2.1 Command audit records are queryable SQLite rows

The CLI audit middleware writes persistent `AuditEvent` records for most top-level commands. These rows live in the configured audit database and back `orbit audit list`, `orbit audit show`, `orbit audit stats`, and export commands.

Command audit rows are compact and queryable. They carry command metadata, target metadata, status, timing, working directory, host, process id, and optional argument/error fields. They do not carry full provider transcripts.

### 2.2 Activity/job run traces are file-backed JSONL trees

The v2 activity/job runtime emits `V2AuditEvent` envelopes for run, step, activity, fan-out, loop, filesystem, denial, and CLI-backend lifecycle events. The current file-backed sink writes these envelopes under `.orbit/state/audit/v2_loop/`.

This layer is the structural spine for workflow replay. It knows `run_id`, `event_id`, `parent_event_id`, `agent_identity`, and optional `workspace_path`.

`orbit run events <run_id>` and `orbit run trace <run_id>` expose this layer as chronological and tree-shaped operator views after [T20260426-0705]. `orbit run show -s <id>` and `orbit run logs -s <id>` use the same activity DAG `step.id` source of truth after [T20260426-0709].

### 2.3 Agent-loop audit events preserve provider and tool detail

The HTTP loop engine emits structured `LoopAuditEvent` records for sessions, HTTP requests/responses, tool requests/results, iteration boundaries, and policy denials. Large payload bodies are stored as redacted content-addressed blobs and referenced by sha256.

Loop-level JSONL lives beside the v2 envelope tree under `.orbit/state/audit/loop/`, with blobs under `.orbit/state/audit/blobs/`.

### 2.4 Invocation metrics are adjacent, not a replacement

The invocation store records token usage, tool-call counts, task IDs, agent, model, job run, and activity IDs for metrics and scoreboards. It is audit-adjacent because it helps answer cost and usage questions, but it is not the canonical transcript.

This distinction became explicit when v2 job metrics started persisting invocation traces beside audit in [T20260426-0526].

### 2.5 Redaction happens before durable payload storage

Blob writes apply pattern-based redaction at write time. CLI error audit paths scrub sensitive live environment values before persistence. Redaction is therefore a write-side guarantee; readers should not need to re-scrub normal audit artifacts.

The redaction and retention contract is specified in [specs/redaction-retention.md](./specs/redaction-retention.md).

### 2.6 Process tracing has a global JSONL feed

The default tracing subscriber appends structured events to `~/.orbit/state/logs/orbit.jsonl` after [T20260426-2343]. After [T20260426-2349], the subscriber redacts string field values, `Debug`-formatted field values, and unstructured messages before writing stderr or JSONL output. This feed is global because subscriber initialization runs before Orbit knows the workspace root, and it is the live, tail-able counterpart to workspace-local run traces.

---

## 3. At a Glance

| Concern | Where it lives | Primary task ID |
|---------|----------------|-----------------|
| Audit design ownership | `docs/design/auditability/` | [T20260426-0605] |
| Command audit record type | `crates/orbit-common/src/types/audit_event.rs` | [T20260426-0605] |
| Command audit middleware | `crates/orbit-cli/src/audit_middleware.rs`, `crates/orbit-cli/src/main.rs` | [T20260426-0605] |
| SQLite audit event store | `crates/orbit-store/src/sqlite/audit_event_store.rs`, `crates/orbit-store/migrations/0001_init.sql` | [T20260426-0605] |
| Audit query/export CLI | `crates/orbit-cli/src/command/audit.rs`, `crates/orbit-core/src/command/audit_event.rs` | [T20260426-0605] |
| V2 activity/job envelope | `crates/orbit-common/src/types/activity_job/audit_envelope.rs` | [T20260419-0002] |
| V2 JSONL writer and sink | `crates/orbit-engine/src/activity_job/audit_writer.rs`, `crates/orbit-engine/src/activity_job/jsonl_sink.rs` | [T20260419-0002], [T20260426-0519] |
| Workspace-local run trace location | `.orbit/state/audit/` | [T20260426-0519] |
| Run trace inspection CLI | `crates/orbit-cli/src/command/run.rs`, `crates/orbit-core/src/runtime/run_audit.rs` | [T20260426-0705], [T20260426-0709] |
| Loop audit events and blob storage | `crates/orbit-agent/src/loop_engine/audit/mod.rs`, `crates/orbit-common/src/utility/blob_store.rs` | [T20260426-0605] |
| Redaction utilities | `crates/orbit-common/src/utility/redaction.rs` | [T20260426-0605] |
| Global tracing JSONL feed | `crates/orbit-common/src/utility/logging.rs`, `~/.orbit/state/logs/orbit.jsonl` | [T20260426-2343], [T20260426-2349] |
| V2 invocation metrics persistence | `crates/orbit-store/src/sqlite/invocation_store.rs`, `crates/orbit-core/src/runtime/v2_host.rs` | [T20260426-0526] |
| Task attribution fields | `crates/orbit-common/src/types/task.rs`, task update/runtime host paths | [T20260426-0605] |

---

## Task References

- **[T20260419-0002]** — Add workspace provenance and v2 audit envelope events for activity/job execution.
- **[T20260426-0519]** — Move file-backed activity/job audit traces under workspace state.
- **[T20260426-0526]** — Persist v2 invocation traces for metrics beside audit.
- **[T20260426-0605]** — Add this auditability design folder and establish codex ownership.
- **[T20260426-0705]** — Expose v2 run audit events through `orbit run events` and `orbit run trace`.
- **[T20260426-0709]** — Align run step selectors on activity `step.id` and move CLI invocation log reading behind orbit-core runtime accessors.
- **[T20260426-2343]** — Add the global process tracing JSONL feed at `~/.orbit/state/logs/orbit.jsonl`.
- **[T20260426-2349]** — Apply tracing-layer redaction before stderr and global JSONL output.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
