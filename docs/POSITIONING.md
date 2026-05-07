# Orbit Positioning

This document names what Orbit is for, who it's for, and how the audience expands over time.

Use it as a decision lens. When a design debate feels like it's really about *what Orbit is for*, reference this doc instead of re-litigating case by case.

## What Orbit is for

**Durable, intent-tracked agentic project management for developers who use AI coding agents heavily — Linear / Jira designed for the AI-native solo developer, with a path to team-scale automation as trust in agents matures.**

The wedge today is the individual engineer driving multiple coding agents against real code and needing the work to outlive any single agent session: a persistent backlog, an audited execution trail, and intent attribution at the codebase level — every line of agent-authored code traceable to the task that produced it. Linear and Jira solve durable project management for human-driven teams. Agent vendors solve in-session execution. Nobody has solved durable, intent-tracked, audited agentic project management for the AI-native solo developer who plans to expand it across their team. Orbit does.

The destination — once trust in agents matures and human review stops being the dominant bottleneck — is fleet orchestration at team scale. That's a multi-year arc, covered in the *Long-arc vision* section below. The wedge does not depend on the destination arriving on any specific timeline.

## Who Orbit is for, in funnel order

Orbit is built for AI-native solo developers who use coding agents heavily, with a deliberate funnel that expands toward teams over time:

1. **The wedge — AI-native solo developers who run multiple coding agents heavily** (Claude Code, Cursor, Aider, Codex CLI) and have outgrown the in-session model. They need durable backlog, lifecycle tracking across sessions and weeks, and intent attribution baked into the codebase itself.

2. **The champion — a subset of (1) with organizational influence**: staff engineers, tech leads, principals, founding engineers. They validate Orbit on their own work, then become the internal advocate for broader team adoption.

3. **The destination — team-scale agentic automation**: coordinated fleet operation across many engineers' machines, eventually a shared-host deployment, with cross-engineer audit aggregation and team-wide scoreboards. Multi-year, not promised on any specific timeline.

These are not three audiences; they are three stages of one audience. We optimize the product for stage 1 (the wedge) because that's where adoption begins, while keeping the architectural path to stage 3 unblocked. Stages 2 and 3 happen because individual engineers carry Orbit into their teams — not because Orbit pitches teams directly.

## What Orbit is NOT for

Two use cases we are deliberately not serving. Each would pull Orbit in a direction that makes it worse for the wedge.

**1. Enterprise procurement.** SOC 2, SSO, multi-tenant permissions, 24/7 support contracts, sales motions, long procurement cycles. Serving these requires headcount, compliance work, and product management discipline incompatible with how Orbit is built. If enterprise demand appears in volume, the honest path is a commercial fork, a partnership, or an acquisition — not bolting enterprise surface onto the OSS core.

**2. Generic workflow orchestration.** n8n, Airflow, LangGraph, Temporal. Orbit is specifically a coding-agent platform, not a generic workflow engine. Features valuable only in non-coding contexts belong elsewhere.

**A note on individual developers.** Claude Code, Cursor, Aider, and Codex CLI solve in-session productivity for individual engineers — one agent in one terminal solving one task. Orbit's wedge starts where their model ends: persistent backlog across sessions, lifecycle tracking across weeks, intent attribution across the codebase. We are not competing with them in-session. We are the layer above — the layer that turns individual agent sessions into a coherent body of work.

## Primary focus: auditability

Auditability is not a cross-cutting concern in Orbit — it is a product feature. Orbit runs agents against code that someone (the operator today, their team tomorrow) has to live with. When something goes wrong (a bad merge, a regression, a mystery refactor), the operator needs to answer three questions without calling the Orbit maintainers:

1. **What did the agent do, exactly?** Every tool call, every provider request/response, every task state transition recorded with enough fidelity to reconstruct the sequence.
2. **Why did it do that?** The prompt, system instructions, role configuration, and surrounding context are recoverable, not just the action.
3. **Who is accountable?** Agent identity, model, provider, and activity context are attached to every event — on commits, on PRs, on audit entries — so a git blame or audit query reaches a concrete agent identity, not "the AI."

Concrete commitments this implies:

- **Complete coverage.** Every operation that touches code, state, or external services emits an audit event. Silent paths are bugs.
- **Structured, queryable events.** Not log strings. Typed records with stable schemas the user can query with `orbit.audit.*` tools and export to their own observability stack.
- **Faithful reproducibility.** Prompts and responses are stored verbatim (with configurable redaction for sensitive paths). Summaries are derived artifacts, not replacements.
- **Tamper-evident retention.** Audit is append-only. The audit trail's own integrity is verifiable; corrupting history should not be a silent operation.
- **Agent-identity attribution.** Every write — commit, PR, audit event, task update — carries the identity of the agent (and model) that produced it. No anonymous AI actions.

When auditability conflicts with performance, ergonomics, or feature surface, auditability wins. Undercutting audit fidelity to save a tool call or a storage row is the kind of decision this doc exists to prevent.

## Non-negotiables

- **Self-hostable.** Single binary, no mandatory cloud dependency. Runs on a laptop, in a container, in a CI runner, behind a firewall.
- **Bring-your-own-credentials.** Orbit never stands between operator and provider. API keys belong to the operator; Orbit is a pass-through.
- **HTTP/SDK-first provider communication.** Programmatic multi-turn is the deployment shape. CLI shell-out is an escape hatch for experimentation, not the backbone.
- **Audit trail for everything that touches code.** See the dedicated *Primary focus: auditability* section above. Non-negotiable and promoted to a product feature, not a compliance concession.
- **Intent attribution at the codebase level.** Every line of agent-authored code is traceable to the task that produced it. `task_id` baked into commit messages, queryable via Orbit's tools, durable across rewrites and refactors. This is what turns a body of agent work into a coherent record over time, instead of a stream of opaque diffs.
- **Reproducibility where possible, recorded non-determinism where not.** Same task + same repo state should converge. When the provider introduces non-determinism, capture it rather than hide it.
- **Knowledge-graph–aware tooling.** Agents operate against a parsed, symbol-level graph of the codebase, not raw grep. This is the technical moat — the reason to pick Orbit over a generic agent framework — and it is measured, not asserted: under MCP exposure, the graph reduces token cost on structural code questions (see `benchmarks/graph/`).
- **Cost-visible.** The operator knows what each run costs in tokens and wall-clock.
- **Git- and GitHub-native.** Branches, worktrees, PRs, CI status. No custom version control abstractions.
- **Configurable, not opinionated to the point of rigidity.** Job DAGs, activity definitions, skill loadouts, role profiles are all data (YAML), not code. Forking is expected; feature requests for narrow customization are not.

`task_id` semantics are locally meaningful by design. It appears in commits as a personal search key for the task author and is recorded in local audit. It is not designed to be resolvable on another engineer's machine; for cross-engineer task references, use `external_refs` to link tasks to your team's tracker (Jira, Linear, GitHub Issues, etc.).

## What Orbit refuses to add

- Hidden cloud dependencies. Orbit must never phone home.
- Vendor lock-in to one LLM provider. Cross-provider is table-stakes.
- Magic. If an agent decision can't be reconstructed, trust erodes permanently.
- Enterprise surface bolted onto OSS core (tenancy, ACLs, SSO) — adds complexity without serving the wedge or its expansion path.
- Onboarding designed for non-technical users — patronizing and misaligned with the audience.
- Subscription-arbitrage architectures that assume a single personal CLI account is the backbone. The wedge audience drives multiple agents in parallel against real code; that breaks the per-account assumption.
- Black-box "just trust the agent" patterns. Every agent decision should be inspectable.

## Long-arc vision: fleet orchestration at team scale

The wedge today is intent-tracked durable agentic project management for the AI-native solo developer. The destination is fleet orchestration at team scale — many agents on many providers running in parallel against shared production code, with cross-agent coordination, team-wide scoreboards, and shared-host deployment aggregating audit and tasks across operators.

This is the bet underneath Orbit's architectural choices. Why we over-build audit, scoreboards, multi-provider primitives, per-agent identity attribution, and policy / sandboxing today, even though the wedge audience could get by with less:

- As trust in agents matures, human review shifts from "every diff" to "exception cases." Throughput becomes the bottleneck, and throughput is rate-limited by safe parallel execution. Orbit's fleet primitives are the substrate for that future state.
- The team-adoption arc — solo champion validates → proposes team adoption → team adopts — is multi-year, especially for teams whose existing culture isn't yet agent-savvy. The product needs to be ready for that arc before demand materializes.
- Self-hosted, BYO-credentials, HTTP/SDK, knowledge-graph-aware: these are decisions optimized for the team-scale destination as much as for the solo wedge. They constrain Orbit's near-term ergonomics in ways that pay off only at the destination.

Concrete capabilities that live in this section, not in the wedge messaging:

- **Fleet primitives, per operator.** Parallel task execution, cross-provider delegation, per-agent scoreboards, per-agent identity in commits — across many agents on one operator's machine. Single-assistant assumptions are incorrect; the architecture treats fleets as the default execution shape.
- **Shared-host deployment (v2, scoped).** One Orbit instance serving multiple operators, aggregating audit, tasks, and scoreboards across the team. Demand for this is downstream of the champion-led team-adoption arc. Architectural unblockers land in v1; the deployment surface itself doesn't ship until demand is real.
- **Team-grade fleet metrics.** Cross-engineer agent throughput, team-wide PR merge rates, multi-operator policy enforcement. Downstream of the shared-host shape.

The wedge does not depend on the destination arriving on any specific timeline. The destination does not change what we sell today.

## The decision lens

When a design decision is contested, the tiebreaker is:

> **Would this hold up for an AI-native solo developer driving multiple agents against real code, AND would it still hold up at team scale once trust matures?**

If the honest answer to either half is no, it doesn't ship.

Secondary lenses, in order:

1. **Does this serve the wedge as much as it serves the destination?** Features that only make sense at team scale, with no benefit to the solo champion, are too far ahead of the audience and shouldn't ship until the audience catches up.
2. **Would this still be true at 10× the current agent fleet size?** If a pattern only works with one orchestrator and three agents, it's wrong.
3. **Can the operator debug this without asking us?** If the answer requires Orbit maintainers in the loop, the feature is under-instrumented.
4. **Does this survive losing confidence in a single provider?** If a design fails when Anthropic rate-limits or deprecates a model, it fails.

## Boundaries (when we'd reconsider)

This positioning is not permanent. We'd reconsider if:

- The wedge audience (AI-native solo developers needing durable agentic project management) turns out not to exist at meaningful scale. Indicator: the 90-day distribution experiments produce zero substantive replies, zero trial users, and no traction across Show HN / blog / outreach.
- Enterprise demand arrives with headcount or funding to serve it properly. The honest response is a commercial arm or partner, not a quiet pivot of the OSS core.
- Trust in agents matures dramatically faster than expected, collapsing the multi-year arc to months. Reframe the destination as a near-term shape rather than a long-arc one, and update messaging accordingly.
- The team-scale destination itself stops being a coherent niche (e.g., team-scale agent automation collapses into solo tooling above and enterprise platforms below). Revisit the whole document in that case — don't patch around it.
- The shared-host deployment shape ships in v2 and demand for it materializes. At that point the per-operator framing in this doc narrows from "what Orbit is" to "what v1 was" — reframe rather than maintain stale per-operator language once shared-host lands.

Until one of those happens, the framing above is the lens.
