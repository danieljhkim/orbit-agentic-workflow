# Auditability — Design

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-26

This document describes Orbit's shipped auditability implementation across command audit rows, activity/job envelopes, loop-level provider/tool traces, blob storage, redaction, identity attribution, metrics-adjacent invocation records, and the current limitations that still need design attention. See [1_overview.md](./1_overview.md) for the feature's purpose and [3_vision.md](./3_vision.md) for forward-looking questions.

---

## 1. Storage Roots and Audit Channels

Auditability is currently split across four channels:

1. **Command audit records.** SQLite rows in the configured audit database. These back the public `orbit audit` query/export surface.
2. **V2 activity/job envelope events.** JSONL files under `.orbit/state/audit/v2_loop/{run_id}.jsonl`.
3. **Loop-level provider/tool events.** JSONL files under `.orbit/state/audit/loop/{run_id}.jsonl`.
4. **Invocation metrics.** SQLite records keyed by job run, activity, task, agent, model, usage, and tool-call summaries.

The split is intentional. Command audit rows are compact and queryable across invocations. Activity/job envelopes preserve workflow structure. Loop audit preserves provider and tool-call detail. Invocation metrics support cost and scoreboard questions without scraping audit JSONL.

The most recent storage-placement decision is [T20260426-0519]: file-backed run traces moved under `.orbit/state/audit/`, while command audit rows remain in the configured SQLite database. README now describes that split as "command audit events remain globally scoped in SQLite; file-backed activity/job run traces are workspace-local under `.orbit/state/audit`."

---

## 2. Command Audit Rows

The command audit schema is `AuditEvent` in `crates/orbit-common/src/types/audit_event.rs`. A row includes:

- `execution_id`
- timestamp
- command and subcommand
- optional tool, target type, and target id
- role
- status: `success`, `failure`, or `denied`
- exit code and duration
- working directory
- optional argument JSON, truncated stdout/stderr, error message, host, pid, and session id

The table and indexes live in `crates/orbit-store/migrations/0001_init.sql`, and the store implementation is `crates/orbit-store/src/sqlite/audit_event_store.rs`. The store can list, show, prune, compute stats, and return durations for p95 calculation.

The CLI path is an RAII guard in `crates/orbit-cli/src/audit_middleware.rs`. `AuditGuard` defaults to failure, marks success or denial explicitly, and writes one row in `Drop`. This means early returns still write an audit record as long as stack unwinding reaches the guard.

`crates/orbit-cli/src/main.rs` wraps non-audit commands in that guard after runtime initialization. Direct `orbit audit ...` commands are deliberately outside the guard today, so querying the audit log does not itself emit another command audit row.

---

## 3. Tool-Driven and Runtime Audit Records

Command audit records do not only come from the top-level CLI guard. Some runtime paths write targeted rows directly:

- `crates/orbit-core/src/runtime/orbit_tool_host/mod.rs` records task lock reservation checks, reservations, releases, and denials with `target_type: task_reservation`.
- `crates/orbit-core/src/runtime/v2_host.rs` records gate-starvation failures for task bundles with `command: gate.starvation`.

These records use the same SQLite schema so `orbit audit list` and export commands can see them. The current design implication is important: audit producers are allowed below the CLI layer, but they must still use `AuditEventInsertParams` and preserve the same status, target, actor, and redaction expectations.

The prescriptive coverage expectations live in [specs/coverage-matrix.md](./specs/coverage-matrix.md).

---

## 4. Activity/Job Envelope Events

The v2 activity/job audit envelope type lives in `crates/orbit-common/src/types/activity_job/audit_envelope.rs`. The envelope fields are:

- `schemaVersion`
- `event_type`
- `event_id`
- timestamp
- `run_id`
- `agent_identity`
- optional `parent_event_id`
- optional `workspace_path`

The body is a tagged `V2AuditEventKind`. Current event families include:

- run lifecycle
- step lifecycle, retry, skip, denial, and join outcomes
- fan-out/fan-in worker state
- loop iteration lifecycle and non-convergence
- activity lifecycle
- filesystem request/result/denial
- tool denial
- CLI-backend allowlist delegation and subprocess lifecycle

`crates/orbit-engine/src/activity_job/audit_writer.rs` owns `V2AuditWriter`. It assigns event ids, stores parent stacks per thread, emits JSONL through `V2JsonlSink`, keeps an in-memory snapshot for smoke verification, and exposes the inner loop sink so provider/tool events can live beneath the envelope tree.

`crates/orbit-engine/src/activity_job/jsonl_sink.rs` writes one JSON object per line under `v2_loop/`, append-only for the life of a run and flushed per write.

`crates/orbit-core/src/runtime/run_audit.rs` owns the read-side accessor for this file-backed layer after [T20260426-0709]. It collects v2 envelope events, derives activity DAG `step.id` values from `parent_event_id` ancestry, and resolves CLI stdout/stderr blob references for `orbit run logs` without exposing audit storage paths to the CLI renderer.

---

## 5. Loop-Level Provider and Tool Events

The lower-level loop audit type is `LoopAuditEvent` in `crates/orbit-agent/src/loop_engine/audit/mod.rs`. Its events cover:

- session spawn and close
- HTTP request and response
- tool-call request and result
- iteration boundary
- policy denial

`JsonlFileSink` writes loop events to `{audit_root}/loop/{run_id}.jsonl`. The same sink writes payload blobs to `{audit_root}/blobs/` and references them by sha256. Orbit runtime callers pass `.orbit/state/audit` as `audit_root`, so loop traces share the same workspace-local audit root as v2 envelopes.

The loop engine emits hashes for request bodies, response bodies, tool inputs, and tool outputs rather than embedding those bodies in the event row. This keeps event lines queryable while still preserving replay material in blob storage.

---

## 6. Blob Storage and Redaction

`crates/orbit-common/src/utility/blob_store.rs` writes content-addressed blobs under `{root}/{hash_prefix}/{hash}`. The hash is computed after redaction, and an existing blob path is reused without rewriting.

`crates/orbit-common/src/utility/redaction.rs` centralizes two redaction mechanisms:

- sensitive live environment values, selected by environment variable name patterns such as `SECRET`, `TOKEN`, `PASSWORD`, `API_KEY`, `AUTH`, and related names
- regex-based HTTP and argv patterns for authorization headers, x-api-key fields, bearer tokens, JSON API keys, and bare `sk-...` tokens when argv scrubbing is requested

The CLI audit guard redacts error messages before writing them. The blob store redacts bytes before writing them. The pipeline runtime also redacts JSON outputs and errors before persisting selected pipeline data.

The smoke example `crates/orbit-agent/examples/redaction_smoke.rs` verifies that stored blob bytes omit the raw secret and contain a redaction marker.

---

## 7. Identity and Attribution

Orbit currently carries identity through several related fields:

- CLI runtime actor identity defaults direct CLI commands to `human`.
- `orbit tool run` paths carry explicit `agent` and `model` inputs for provenance.
- `V2AuditEnvelope.agent_identity` records the actor label used for an activity/job run.
- Task records carry `created_by`, `planned_by`, `implemented_by`, `agent`, and `model` fields.
- Invocation metrics record agent and model beside job run and activity ids.

The core design requirement is not that all of these fields collapse into one value. It is that a reviewer can follow the chain from task state, command rows, v2 run envelope, provider/tool trace, and metrics record back to a concrete human or model identity.

This area is still uneven. Some paths store role strings such as `admin`; some store actor labels; some normalize model names into attribution labels. The design intent is clear, but a unified identity glossary and query join story remain open.

---

## 8. Query, Export, and Metrics Surfaces

`crates/orbit-cli/src/command/audit.rs` exposes command audit rows through:

- `orbit audit list`
- `orbit audit show <id>`
- `orbit audit stats`
- `orbit audit export --format json`
- `orbit audit export --format csv`
- `orbit audit prune`

The command audit query surface supports filters for time, tool, status, role, and limit. Exports include all command-audit columns, including currently sparse fields such as `stdout_truncated`, `stderr_truncated`, and `session_id`.

V2 JSONL traces are exposed through run-scoped inspection commands after [T20260426-0705]:

- `orbit run events [run_id]` prints chronological envelope events and supports step and event-type filters.
- `orbit run trace [run_id]` renders the `event_id` / `parent_event_id` tree.
- `orbit run logs [run_id]` extracts CLI stdout/stderr blobs from CLI invocation envelope events.

These are intentionally separate from `orbit audit`, which remains the compact SQLite command-audit query surface. `orbit job history` and `orbit job run-state` continue to expose durable job-run state rather than the full envelope stream.

Invocation metrics are surfaced through metrics and scoreboard commands. They are useful for cost and usage analysis, but they do not replace the audit trail because they summarize rather than preserve transcript structure.

---

## 9. Concerns & Honest Limitations

1. **Tamper evidence is promised more strongly than it is implemented.** README describes append-only and tamper-evident retention, but the current SQLite rows and JSONL files do not yet have a hash chain, signature, or external transparency log.
2. **Audit is split across storage systems.** Command audit rows, v2 JSONL, loop JSONL, blobs, job-run state, and invocation metrics are related by ids but not yet joined by one operator command.
3. **`orbit audit` does not audit itself.** That avoids recursive noise, but it also means audit-log reads, exports, and prunes are not themselves recorded through the normal command guard.
4. **Some command-audit fields are placeholders.** `stdout_truncated`, `stderr_truncated`, and `session_id` exist in the schema but are often `None`.
5. **CLI backend tool enforcement remains weaker than HTTP.** Activity/job audit records the CLI backend allowlist as harness-delegated. That preserves accountability but does not enforce Orbit-level tool denial semantics in the CLI provider path.
6. **Redaction favors known secret shapes.** Environment-value and regex redaction cover common provider-key paths, but no redactor can prove arbitrary user secrets are absent from every payload.
7. **Coverage is still expanding.** Some deterministic actions and direct runtime mutations write explicit audit rows; others rely on enclosing command/job context. The coverage matrix must become the review checklist for new mutation paths.

---

## Task References

- **[T20260419-0002]** — Add workspace provenance and v2 audit envelope events for activity/job execution.
- **[T20260426-0519]** — Move file-backed activity/job audit traces under workspace state.
- **[T20260426-0526]** — Persist v2 invocation traces for metrics beside audit.
- **[T20260426-0605]** — Add this auditability design folder and document the current audit architecture.
- **[T20260426-0705]** — Expose v2 run audit events through `orbit run events` and `orbit run trace`.
- **[T20260426-0709]** — Align run step selectors on activity `step.id` and move CLI invocation log reading behind orbit-core runtime accessors.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
