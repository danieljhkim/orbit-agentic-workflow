# Activity / Job — Vision

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-21

This document captures where the Activity / Job substrate may go next. It starts from the current v2 runtime shipped across [T20260418-2010], [T20260418-2018], [T20260418-2143], [T20260418-2210], [T20260419-0104], and [T20260419-2156], and treats everything below as pressure-testing rather than commitment. [2_design.md](./2_design.md) is the current contract; this file is where we ask which parts of that contract should harden, collapse, or disappear.

---

## 1. Open Questions

### 1.1 How much of Activity / Job should remain public?

README already frames tasks, jobs, and activities as substrate rather than the long-term product story. Should Orbit eventually keep this layer mostly internal, with goals/sessions/locks becoming the user-facing API, or does a durable local system still need a first-class human-editable workflow substrate?

### 1.2 Should `target:` grow beyond `activity:<name>`?

`TargetRef` is namespace-prefixed on purpose. Right now only `activity:<name>` exists, from [T20260418-2019]. Do we want nested job refs, reusable subroutine refs, or cross-workspace refs, or is that the point where this layer turns back into a generic workflow engine?

### 1.3 Is the provider model too closed-set?

The schema names multiple providers, but the HTTP transport path is still effectively single-provider today. Do we keep the closed enum and wire each transport deliberately, or move toward a more explicit "provider capability" registry?

### 1.4 Should Orbit enforce tool allowlists on the CLI backend?

The current gap from [T20260419-0104] is honest but significant: HTTP enforces, CLI advises. Is that acceptable long term, or does `backend: cli` need a stricter wrapper so the meaning of `tools:` stops depending on transport choice?

### 1.5 Which limits should stay structural literals?

Real shipped jobs like `task_auto_pipeline` and `task_epic_pipeline` rely on literal `max_workers` and `max_iterations`. Should those remain static authoring-time choices, or do we need templated numerics and dynamically computed ceilings?

### 1.6 What is the right audit landing zone?

The v2 envelope from [T20260419-0002] is useful because it makes runs traversable by step/activity structure, but the full HTTP transcript still lives in the sibling loop sink. Should the reviewer-facing story converge on one query surface even if the files remain separate?

### 1.7 How much more special-casing belongs in ActivityV2?

Groundhog's dedicated activity kind from [T20260420-0510-2] is justified today. If more execution modes arrive, do we keep adding sibling variants, or is there a point where "activity kind explosion" means the underlying abstraction has gone stale?

### 1.8 What should become the canonical reference corpus?

The seeded assets added in [T20260419-2347] and extended in [T20260419-0622-3], [T20260419-0623], and [T20260419-0623-2] already function as executable docs. Should Orbit lean into that and treat seeded jobs as the main reference set, or do we need smaller spec-only exemplars for every construct?

---

## 2. Prior Work

Activity / Job is a productization of familiar ideas, not a novel research primitive. The interesting question is whether Orbit's combination is locally inspectable and agent-friendly enough to matter in practice.

### 2.1 Durable workflow engines

- **Temporal** treats replay-safe orchestration and activity boundaries as first-class runtime concepts.
- **Argo Workflows** and **GitHub Actions** show that human-authored YAML orchestration is still valuable even when higher-level product abstractions exist.
- **LangGraph** has made durable, stateful control flow part of the mainstream agent-tooling vocabulary.

Activity / Job borrows the idea that orchestration must survive outside one model turn, but keeps the runtime local-first and repo-scoped.

### 2.2 Agent runtimes and tool loops

- **Claude Code**, **Codex**, **Gemini CLI**, and similar tools normalize the "agent loop plus tools" model.
- **OpenAI-compatible** and provider-native HTTP APIs normalize direct transport-level loops.

Orbit's difference is not that it can run a tool-using loop. It is that the loop is one typed activity inside a larger durable runtime with local audit, policy, and job control-flow.

### 2.3 Policy-aware execution substrates

- Sandbox and policy systems in CI/CD platforms establish the norm that execution policy is part of the runtime contract, not just ambient environment.
- Agent harnesses frequently expose allowlists, but many treat them as soft guidance rather than a load-bearing runtime surface.

Orbit's `fsProfile` attachment and the HTTP/CLI enforcement split make this concern explicit, even if the current behavior is still uneven.

### 2.4 Executable reference assets

- Mature workflow systems almost always ship example pipelines, starter templates, or scaffolded defaults.

Orbit's seeded activities and jobs from [T20260419-2347] are already acting as more than examples. They are the closest thing the project has to an executable specification corpus.

---

## 3. What May Be Distinctive

Soft claims only:

- **Load-time normalization as a public contract.** Target-ref resolution, backend concretization, and loop/session rejection are part of what a job *is*, not just hidden parser details.
- **Backend choice separated from provider choice.** Orbit treats `backend: http|cli|auto` and `provider: ...` as orthogonal schema fields, then makes mismatches explicit instead of silently recovering.
- **A two-layer audit tree tied to repo provenance.** The v2 envelope from [T20260419-0002] gives runs, steps, activities, and workspace origin a stable skeleton while still preserving the underlying loop transcript and blobs.
- **Seeded workflows as load-bearing contracts.** The shipped jobs from [T20260419-0622-3], [T20260419-0623], and [T20260419-0623-2] are not toy examples; they are how the control-flow substrate proves itself against real Orbit work.

None of these are research contributions. If Activity / Job earns its keep, it will be because the layer stays understandable, local, and inspectable while the product around it evolves.

---

## 4. References

### Orbit-internal

- [1_overview.md](./1_overview.md) — feature purpose and core concepts
- [2_design.md](./2_design.md) — current implementation
- [4_decisions.md](./4_decisions.md) — ADR log
- [specs/backend-resolution.md](./specs/backend-resolution.md) — backend concretization and HTTP-only session rule
- [specs/audit-envelope.md](./specs/audit-envelope.md) — v2 audit event tree and persistence layout
- [../groundhog/1_overview.md](../groundhog/1_overview.md) — sibling activity kind built on this substrate
- [../knowledge-graph/1_overview.md](../knowledge-graph/1_overview.md) — graph substrate that sits beside this execution substrate

### External

- Temporal — https://temporal.io/
- Argo Workflows — https://argo-workflows.readthedocs.io/
- GitHub Actions — https://docs.github.com/actions
- LangGraph — https://langchain-ai.github.io/langgraph/

---

## Task References

- **[T20260418-2010]** — Add the first v2 activity runtime scaffolding.
- **[T20260418-2018]** — Add `JobV2` DAG constructs (`parallel`, `fan_out`, `loop`, `retry`, `when`).
- **[T20260418-2019]** — Add v2 activity name resolution and pipeline skeleton assets.
- **[T20260418-2143]** — Wire `V2RuntimeHost` in orbit-core and add `orbit activity run-v2`.
- **[T20260418-2210]** — Reshape `V2RuntimeHost` to keep `orbit-agent` types out of orbit-core.
- **[T20260419-0002]** — Add `workspace_path` provenance to the v2 audit envelope.
- **[T20260419-0104]** — Add `backend: cli` dispatch for v2 `agent_loop`.
- **[T20260419-0622-3]** — Add `task_gate_pipeline`.
- **[T20260419-0623]** — Add `task_auto_pipeline`.
- **[T20260419-0623-2]** — Add `task_epic_pipeline`.
- **[T20260419-2156]** — Retire v1 assets and drop the transitional v2 naming.
- **[T20260419-2347]** — Seed activities and workflows on `orbit init`.
- **[T20260420-0510-2]** — Add the Groundhog v1 activity runner.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
