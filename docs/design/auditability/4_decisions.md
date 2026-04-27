# Auditability — Decisions

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-26

This is the append-only ADR log for Auditability. Entries are ordered by ADR number. New entries should use the template in [../CONVENTIONS.md](../CONVENTIONS.md) and cite the task that made the decision real.

---

## ADR-001 — Dedicated auditability design ownership

**Status:** Accepted · 2026-04 · [T20260426-0605]

**Context.** Auditability is named as a primary Orbit feature, but the implementation and rationale were spread across README prose, Activity / Job docs, SQLite audit code, loop audit code, and redaction utilities.

**Decision.** Create `docs/design/auditability/` as the canonical design folder for auditability, with codex as owner.

**Consequences.**
- Audit decisions now have one ADR log and one glossary.
- Future audit coverage work can cite a feature-owned spec rather than copying README promises.
- Cost: auditability now overlaps with Activity / Job docs, so cross-links must stay current rather than duplicating the full v2 runtime design.

## ADR-002 — Command audit rows stay compact and queryable

**Status:** Accepted · 2026-04 · [T20260426-0605]

**Context.** CLI commands need a durable, filterable history across process invocations, but stuffing full provider payloads into command rows would make routine audit queries noisy and expensive.

**Decision.** Keep command audit records as compact SQLite rows with command, target, role, status, timing, working directory, and optional argument/error fields. Store transcript-level detail in run-trace JSONL and blobs instead.

**Consequences.**
- `orbit audit list/show/stats/export` can stay fast and table-shaped.
- Full replay data has a separate home better suited to append-only files and content-addressed blobs.
- Cost: reconstructing a complete incident can require joining command rows with job state and file-backed traces.

## ADR-003 — V2 run structure and loop transcript detail are separate audit layers

**Status:** Accepted · 2026-04 · [T20260419-0002]

**Context.** Activity/job execution needs run, step, retry, fan-out, loop, and activity structure. Provider loops need HTTP, tool-call, payload, and session detail. One event type cannot serve both needs cleanly.

**Decision.** Use `V2AuditEnvelope` for activity/job structure and keep `LoopAuditEvent` for provider/tool detail. Connect the layers through run ids and parent event ids rather than merging them into one schema.

**Consequences.**
- Workflow replay can traverse a run tree without loading every provider payload.
- Loop-level audit can evolve with provider/tool semantics without changing the job DAG envelope.
- Cost: reviewers need tooling or documentation to move between related files.

## ADR-004 — File-backed run traces are workspace-local state

**Status:** Accepted · 2026-04 · [T20260426-0519]

**Context.** V2 JSONL and blob traces were runtime artifacts, but they previously lived under a first-level `.orbit/audit/` path that blurred command audit, workspace state, and durable authoring surfaces.

**Decision.** Store activity/job envelopes, loop events, and blobs under `.orbit/state/audit/`, while command audit rows remain in the configured SQLite audit database.

**Consequences.**
- Runtime traces live with other workspace-local run state.
- The file layout distinguishes command audit queries from run reconstruction artifacts.
- Cost: old local `.orbit/audit/` artifacts may require manual fallback or migration if a user wants historical run reconstruction.

## ADR-005 — Redaction is a write-side durability boundary

**Status:** Accepted · 2026-04 · [T20260426-0605]

**Context.** Audit needs faithful payloads for reproducibility, but storing raw provider keys or sensitive environment-derived values would make the audit trail unsafe by default.

**Decision.** Redact sensitive env values, HTTP authorization patterns, API-key fields, bearer tokens, and selected argv token shapes before durable blob or error-message persistence.

**Consequences.**
- Audit readers can treat normal stored blobs as already redacted.
- Smoke tests can verify stored bytes, not just display output.
- Cost: redaction changes payload hashes and may remove exact bytes that would otherwise help reproduce a provider interaction.

## ADR-006 — Invocation metrics are audit-adjacent primary records

**Status:** Accepted · 2026-04 · [T20260426-0526]

**Context.** V2 job execution emitted audit JSONL, but metrics and scoreboards read the invocation store. Deriving metrics by scraping audit logs would couple operator reporting to provider transcript format and JSONL retention.

**Decision.** Persist `InvocationTrace` records beside audit as first-class metric records keyed by job run, activity, task ids, agent, model, usage, and tool-call summaries.

**Consequences.**
- `orbit metrics` and scoreboards do not need to parse audit JSONL.
- CLI and HTTP agent-loop paths can converge on one usage record shape.
- Cost: job execution has another persistence side effect, and metrics can diverge from transcript detail if a provider path reports incomplete usage.

## ADR-007 — Run trace inspection stays separate from command audit

**Status:** Accepted · 2026-04 · [T20260426-0705], [T20260426-0709]

**Context.** Operators need first-class commands for activity/job envelope JSONL, but `orbit audit` is intentionally the compact SQLite command-audit surface. Mixing run-local envelope traversal into command-audit queries would blur two storage scopes and make command audit rows carry workflow-specific semantics.

**Decision.** Expose v2 envelope inspection under `orbit run events` and `orbit run trace`, and keep `orbit audit` focused on command-audit rows. Keep envelope JSONL/blob parsing behind orbit-core runtime accessors so CLI rendering does not own file-backed run-trace layout.

**Consequences.**
- Operators can inspect both command history and run-local workflow traces through dedicated commands.
- Activity `step.id` becomes the shared selector for `run show`, `run logs`, and run trace/event inspection.
- Cost: users must understand that `orbit audit` and `orbit run events/trace` answer related but different audit questions.

## ADR-008 — Process tracing feed is global JSONL

**Status:** Accepted · 2026-04 · [T20260426-2343]

**Context.** CLI subprocess output now emits structured tracing events after [T20260426-2313], but subscriber initialization happens before Orbit resolves a workspace root.

**Decision.** Append process-level tracing events to `~/.orbit/state/logs/orbit.jsonl` through the default subscriber, using the same `EnvFilter` as stderr and a non-blocking file writer retained for the process lifetime.

**Consequences.**
- Operators and future dashboards can tail one machine-readable feed across workspaces.
- Early bootstrap events have a durable destination without needing runtime path resolution.
- Cost: the v1 file is unrotated and concurrent processes can rarely interleave oversized JSONL records.

## ADR-009 — Tracing redaction is enforced by field formatting

**Status:** Accepted · 2026-04 · [T20260426-2349]

**Context.** The global JSONL feed made `tracing` output durable, but pre-emission helpers such as `redact_event_text` only protected call sites that remembered to use them.

**Decision.** Install a redacting `FormatFields` implementation on both stderr and JSONL tracing formatters. The formatter redacts string field values, `Debug`-formatted field values, and unstructured `message` output while preserving field names and typed numeric/boolean JSON values.

**Consequences.**
- New structured tracing emitters inherit the default redaction path before data reaches terminal or disk output.
- CLI subprocess tracing can emit raw line fields while the retained stdout/stderr audit blobs preserve original bytes.
- Cost: span attribute redaction, binary payload redaction, and user-configurable redaction policies remain separate follow-up concerns.

---

## Task References

- **[T20260419-0002]** — Add workspace provenance and v2 audit envelope events for activity/job execution.
- **[T20260426-0519]** — Move file-backed activity/job audit traces under workspace state.
- **[T20260426-0526]** — Persist v2 invocation traces for metrics beside audit.
- **[T20260426-0605]** — Add this auditability design folder and record initial ADRs.
- **[T20260426-0705]** — Expose v2 run audit events through `orbit run events` and `orbit run trace`.
- **[T20260426-0709]** — Align run step selectors on activity `step.id` and move CLI invocation log reading behind orbit-core runtime accessors.
- **[T20260426-2313]** — Stream CLI subprocess stdout/stderr through structured tracing events.
- **[T20260426-2343]** — Add the global process tracing JSONL feed at `~/.orbit/state/logs/orbit.jsonl`.
- **[T20260426-2349]** — Apply tracing-layer redaction before stderr and global JSONL output.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
