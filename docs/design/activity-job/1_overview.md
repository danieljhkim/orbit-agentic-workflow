# Activity / Job — Overview

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-21

> *"Activities are Orbit's executable verbs. Jobs are the control-flow grammar that composes those verbs into durable, inspectable runs."*

Activity / Job is Orbit's current execution substrate. An activity describes one runnable unit. A job describes how activities compose: sequentially, in parallel, across collections, or through bounded loops. Orbit's product story is moving upward toward goals, graphs, sessions, and locks, but this layer remains the load-bearing runtime underneath. [2_design.md](./2_design.md) describes the current implementation; [3_vision.md](./3_vision.md) captures the open questions and the likely simplifications ahead.

---

## 1. Motivation

Orbit needs a runtime layer that is explicit enough for humans to inspect and rigid enough for code to execute. Activity / Job exists to solve four practical problems:

1. **Give agent work a typed execution surface.** Orbit needs more than "run a prompt." It needs agent loops, deterministic actions, shell steps, and now Groundhog attempts, all represented in one schema family. The first v2 activity runtime scaffolding landed in [T20260418-2010].
2. **Make control flow durable and local.** Retry, parallelism, fan-out, and bounded loops must survive outside a single model turn. Phase 3's `JobV2` DAG constructs landed in [T20260418-2018] for exactly this reason.
3. **Keep core/runtime boundaries honest.** orbit-core should coordinate runs without naming `orbit-agent` internals. The `V2RuntimeHost` boundary was wired in [T20260418-2143] and tightened in [T20260418-2210].
4. **Retire the older mixed runtime without guessing at migration.** `schemaVersion: 1` assets now fail load-time parsing after [T20260419-2156], so this v2 surface is no longer "the new path"; it is the canonical one.

---

## 2. Core Concepts

### 2.1 Activities are the runnable units

An `ActivityV2` carries shared metadata plus one concrete runtime spec:

- `agent_loop`
- `groundhog`
- `deterministic`
- `shell`

The shared shape and first dispatcher path shipped in [T20260418-2010]. Groundhog joined as a sibling activity kind in [T20260420-0510-2], rather than becoming another `agent_loop` flag.

### 2.2 Jobs are the orchestration grammar

A `JobV2` is a step tree with:

- `when`
- `retry`
- flat target steps
- `target: activity:<name>` references
- `parallel`
- `fan_out` / `fan_in`
- `loop`

That grammar landed in [T20260418-2018], with `workflow` / `subroutine` job kinds added in [T20260419-0339].

### 2.3 Load-time normalization is part of the contract

The Activity / Job layer does real work before a run starts. orbit-core:

- loads YAML through a two-pass schema loader
- resolves `target: activity:<name>` references for jobs
- rewrites `backend: auto` to a concrete backend once per run
- rejects loop/session/backend combinations that cannot execute safely

The job-side name-resolution pass arrived in [T20260418-2019]. The concrete backend resolution and `run-v2` entrypoints were wired in [T20260418-2143], and CLI backend support followed in [T20260419-0104].

### 2.4 Backends and providers are separate choices

For `agent_loop`, Orbit distinguishes:

- **backend**: `http`, `cli`, or `auto`
- **provider**: `claude`, `codex`, `gemini`, `ollama`, or `openai_compat`

That distinction matters operationally. `backend: auto` resolves once, at load time. `backend: http` against an unwired provider fails structurally rather than silently falling back. `backend: cli` retains the older CLI-provider runtimes on purpose, per [T20260419-0104] and the boundary cleanup in [T20260418-2210].

### 2.5 Audit, policy, and seeded assets make the runtime inspectable

This layer is not just types plus executors. It also owns:

- `fsProfile` attachment on activities and target steps
- the v2 audit envelope with `workspace_path` provenance
- seeded reference assets and pipeline jobs used by `orbit init`

`workspace_path` entered the envelope in [T20260419-0002]. Runtime/CLI `fsProfile` enforcement landed in [T20260419-0503]. Seeding the activity/job assets on init landed in [T20260419-2347].

---

## 3. At a Glance

| Concern | Where it lives | Primary task ID |
|---------|----------------|-----------------|
| v2 activity type system | `crates/orbit-common/src/types/activity_job/activity_v2.rs` | [T20260418-2010] |
| v2 job step grammar | `crates/orbit-common/src/types/activity_job/job_v2.rs` | [T20260418-2018] |
| Job kinds (`workflow`, `subroutine`) | `crates/orbit-common/src/types/activity_job/job_v2.rs` | [T20260419-0339] |
| Target-ref resolution | `crates/orbit-common/src/types/activity_job/catalog.rs` | [T20260418-2019] |
| `run-v2` core entrypoints and host boundary | `crates/orbit-core/src/command/activity_v2.rs`, `crates/orbit-core/src/command/job_v2.rs` | [T20260418-2143], [T20260418-2210] |
| Backend resolution and loop/session constraints | `crates/orbit-core/src/command/backend_resolver.rs`, `crates/orbit-common/src/types/activity_job/backend.rs` | [T20260419-0104] |
| v2 DAG executor | `crates/orbit-engine/src/activity_job/job_executor.rs` | [T20260418-2018] |
| V2 audit envelope and disk sink | `crates/orbit-common/src/types/activity_job/audit_envelope.rs`, `crates/orbit-engine/src/activity_job/audit_writer.rs` | [T20260419-0002] |
| `backend: cli` runtime path | `crates/orbit-engine/src/activity_job/cli_runner.rs` | [T20260419-0104] |
| `fsProfile` enforcement | `crates/orbit-policy`, `tool_context_for_activity`, CLI describe/get surfaces | [T20260419-0503] |
| Seeded reference activities and pipeline jobs | `crates/orbit-core/assets/activities/`, `crates/orbit-core/assets/jobs/` | [T20260419-2347], [T20260419-0622-3], [T20260419-0623], [T20260419-0623-2] |
| Groundhog as a sibling activity kind | `crates/orbit-engine/src/activity_job/groundhog.rs` | [T20260420-0510-2] |

---

## Task References

- **[T20260418-2010]** — Add the first v2 activity runtime scaffolding.
- **[T20260418-2018]** — Add `JobV2` DAG constructs (`parallel`, `fan_out`, `loop`, `retry`, `when`).
- **[T20260418-2019]** — Add v2 activity name resolution and pipeline skeleton assets.
- **[T20260418-2143]** — Wire `V2RuntimeHost` in orbit-core and add `orbit activity run-v2`.
- **[T20260418-2210]** — Reshape `V2RuntimeHost` to keep `orbit-agent` types out of orbit-core.
- **[T20260419-0002]** — Add `workspace_path` provenance to the v2 audit envelope.
- **[T20260419-0104]** — Add `backend: cli` dispatch for v2 `agent_loop`.
- **[T20260419-0339]** — Add v2 job kinds to the job catalog.
- **[T20260419-0503]** — Enforce `fsProfile` rules across runtime and CLI surfaces.
- **[T20260419-0622-3]** — Add `task_gate_pipeline`.
- **[T20260419-0623]** — Add `task_auto_pipeline`.
- **[T20260419-0623-2]** — Add `task_epic_pipeline`.
- **[T20260419-2156]** — Retire v1 assets and drop the transitional v2 naming.
- **[T20260419-2347]** — Seed activities and workflows on `orbit init`.
- **[T20260420-0510-2]** — Add the Groundhog v1 activity runner.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
