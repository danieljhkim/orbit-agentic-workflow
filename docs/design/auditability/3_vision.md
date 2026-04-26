# Auditability — Vision

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-26

This document captures the questions that need to be answered before Orbit's auditability story matches its product promise end to end. It is intentionally forward-looking: [2_design.md](./2_design.md) is the current implementation, while this file names the pressure points that should drive future tasks and ADRs.

---

## 1. Open Questions

1. **What is the canonical audit query surface?** Should `orbit audit` remain command-row-only, or should it grow a run-centric view that joins command audit rows, job-run state, v2 envelope JSONL, loop JSONL, blobs, and invocation metrics?
2. **How should tamper evidence work locally?** Options include per-file hash chains, SQLite append proofs, signed manifests, git-backed checkpoints, or export-time attestations. The answer must preserve Orbit's self-hosted/no-cloud constraint.
3. **Should audit-log reads and prunes be audited?** `orbit audit` is currently outside the normal command guard. That avoids recursion but weakens accountability for audit export and prune operations.
4. **What is the stable identity key?** Orbit needs a joinable identity story across human CLI usage, `agent`/`model` tool inputs, task attribution fields, v2 `agent_identity`, invocation metrics, commits, and PR metadata.
5. **How complete should stdout/stderr capture be?** The command-audit schema has truncated stdout/stderr fields, but most paths do not populate them. Orbit needs a retention policy before filling them broadly.
6. **How should file-backed JSONL migrate?** [T20260426-0519] moved run traces to `.orbit/state/audit/`; any future reader should decide whether old `.orbit/audit/` files are migrated, ignored, or read through a legacy fallback.
7. **How much payload should be replayable?** Faithful reproducibility argues for storing prompts and responses. Secret minimization argues for smaller, redacted payloads. The long-term contract needs to say when summaries are enough and when verbatim redacted payloads are required.
8. **Can policy denials become a uniform audit shape?** Filesystem denials, tool allowlist denials, task-lock conflicts, and gate starvation all exist, but they do not yet share one event schema.
9. **What is the operator story for coverage gaps?** New mutation paths should fail review if they lack audit coverage, but the repo needs mechanical lint or tests before that expectation scales.
10. **Should invocation metrics be derived or primary?** Today they are separate primary records for metrics. A future system could derive them from loop traces, but doing so would increase coupling and reduce resilience to provider-specific output gaps.

---

## 2. Prior Work

### 2.1 Orbit-Internal

The closest internal design work is Activity / Job's audit-envelope section and ADRs in [../activity-job/2_design.md](../activity-job/2_design.md) and [../activity-job/4_decisions.md](../activity-job/4_decisions.md). That feature documents the v2 envelope, backend differences, CLI allowlist delegation, file-backed audit placement, and invocation trace persistence.

The product contract lives in [../../../README.md](../../../README.md) and [../../POSITIONING.md](../../POSITIONING.md): auditability should answer what, why, and who; audit rows should be structured and queryable; provider interactions should be faithfully reproducible after redaction; retention should become tamper-evident; and identity should attach to every write.

The redaction and blob-store implementation gives Orbit an existing write-side safety boundary. That boundary should be preserved even if future query tools read blobs more directly.

### 2.2 Workflow and Observability Systems

Durable workflow engines such as Temporal and Airflow show the value of explicit run, step, retry, and state-transition history. Orbit's distinction is that provider turns, code tools, filesystem policy, and git/worktree state are first-class audit subjects rather than opaque task logs.

OpenTelemetry and structured logging systems show the value of trace/span identifiers and exportable schemas. Orbit can borrow the join model without turning audit into generic telemetry; audit events carry accountability and replay obligations that metrics do not.

### 2.3 Supply-Chain Attestation

SLSA, in-toto, Sigstore, and Rekor point toward signed or transparency-backed provenance. Orbit should learn from those systems, especially for tamper evidence, but it must keep local/self-hosted operation as the default.

### 2.4 Security Event Logging

Security audit systems treat reads, writes, privilege decisions, and retention as part of one accountability surface. Orbit has a narrower domain, but the principle applies: a policy denial, a task update, a provider request, and a PR operation should all be explainable from the audit trail.

---

## 3. What May Be Distinctive

1. **Code-aware audit joins.** Orbit can join audit records not only by run id, but by task id, context selector, lock reservation, worktree, graph node, and commit attribution.
2. **Agent identity as a first-class actor.** The audit trail can name humans, model families, provider backends, and task roles without pretending an agent is just a service account.
3. **Transcript plus structure.** V2 envelopes describe the workflow tree, while loop blobs preserve redacted provider/tool payloads. Most systems pick either high-level structure or low-level logs; Orbit needs both.
4. **Local tamper evidence.** A self-hosted team-scale runtime can produce verifiable local audit bundles without requiring a hosted control plane.
5. **Coverage as a design gate.** New Orbit mutation paths should have to name their audit event family before they ship, just as new activity/job semantics now need ADR coverage.

---

## 4. References

Orbit-internal:

- [1_overview.md](./1_overview.md) — auditability purpose and current concept map.
- [2_design.md](./2_design.md) — shipped implementation and limitations.
- [specs/event-schema.md](./specs/event-schema.md) — prescriptive event-channel contract.
- [specs/coverage-matrix.md](./specs/coverage-matrix.md) — expected audit coverage by operation class.
- [specs/redaction-retention.md](./specs/redaction-retention.md) — write-side redaction and retention boundaries.
- [../activity-job/specs/audit-envelope.md](../activity-job/specs/audit-envelope.md) — existing v2 activity/job audit envelope spec.
- [../../../README.md](../../../README.md) — product-level auditability commitments and public status.
- [../../POSITIONING.md](../../POSITIONING.md) — decision lens for auditability as a non-negotiable.

External reference categories:

- Workflow run history: Temporal, Airflow.
- Structured traces and logs: OpenTelemetry.
- Supply-chain provenance and tamper evidence: SLSA, in-toto, Sigstore, Rekor.
- Security audit logging: append-only security event logs and retention policies.

---

## Task References

- **[T20260419-0002]** — Add workspace provenance and v2 audit envelope events for activity/job execution.
- **[T20260426-0519]** — Move file-backed activity/job audit traces under workspace state.
- **[T20260426-0526]** — Persist v2 invocation traces for metrics beside audit.
- **[T20260426-0605]** — Add this auditability design folder and name future auditability questions.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
