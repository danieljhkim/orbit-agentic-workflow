# Orbit Positioning

This document names what Orbit is for, who it's for, and how the audience expands over time.

Use it as a decision lens. When a design debate feels like it's really about *what Orbit is for*, reference this doc instead of re-litigating case by case.

## What Orbit is for

**Durable, intent-tracked agentic project management for developers who use AI coding agents heavily — Linear / Jira designed for the AI-native solo developer, with a path to team-scale automation as trust in agents matures.**

The wedge today is the individual engineer driving multiple coding agents against real code and needing the work to outlive any single agent session: a persistent backlog, an audited execution trail, and intent attribution at the codebase level — every line of agent-authored code traceable to the task that produced it. Linear and Jira solve durable project management for human-driven teams. Agent vendors solve in-session execution. Nobody has solved durable, intent-tracked, audited agentic project management for the AI-native solo developer who plans to expand it across their team. Orbit does.

The destination — once trust in agents matures and human review stops being the dominant bottleneck — is fleet orchestration at team scale, served by Orbit's hosted team product (a separate paid SKU). That's a multi-year arc, covered in the *Commercial roadmap* section below. The wedge does not depend on the destination arriving on any specific timeline.

## Commercial model: open-core, two tiers

Orbit ships in two tiers. The split is locked at the architecture level (separate repositories), not as feature flags or license keys in shared code.

- **Orbit OSS (the wedge tier).** Self-hosted, single-operator. Agent loop, knowledge graph, audit, task layer, MCP, providers, CLI. Ships under a permissive license (MIT or Apache 2.0). Free forever for individuals and small teams running their own infrastructure. The OSS is self-sufficient: no single-operator workflow is ever gated behind the paid tier.
- **Orbit Team (the paid tier, in development).** Hosted multi-tenant deployment serving an engineering organization. Cross-engineer audit aggregation, team scoreboards, SSO/SAML, RBAC, hosted operations, support SLAs. Closed-source SaaS, separate repository, separate billing.

The boundary between tiers is locked by one rule: *"would a solo developer running self-hosted Orbit on their laptop want this?"* — yes → OSS, no → Team. Apply this rule consistently and the boundary doesn't drift over time.

This is the same open-core pattern Sentry, GitLab, PostHog, Supabase, and HashiCorp use. The OSS is the marketing; the hosted Team product is the business.

## Who Orbit is for, in funnel order

Orbit is built for AI-native solo developers who use coding agents heavily, with a deliberate funnel that expands toward teams over time:

1. **The wedge — AI-native solo developers who run multiple coding agents heavily** (Claude Code, Cursor, Aider, Codex CLI) and have outgrown the in-session model. They need durable backlog, lifecycle tracking across sessions and weeks, and intent attribution baked into the codebase itself.

2. **The champion — a subset of (1) with organizational influence**: staff engineers, tech leads, principals, founding engineers. They validate Orbit on their own work, then become the internal advocate for broader team adoption.

3. **The destination — team-scale agentic automation, served by Orbit Team (the paid hosted product)**: coordinated fleet operation across many engineers' machines, with cross-engineer audit aggregation, team scoreboards, SSO, RBAC. The commercial conversion point: champion proposes adoption → company buys hosted Orbit Team. Target segment is growth-stage and mid-market organizations (10-500 engineers); true Fortune-500 enterprise is a multi-year-out concern, not a near-term target.

These are not three audiences; they are three stages of one audience. We optimize the OSS product for stage 1 (the wedge) because that's where adoption begins. Stage 3 — the paid commercial conversion — happens because individual engineers carry Orbit into their teams, not because Orbit pitches teams directly. The OSS funnels into the Team product; the Team product funds the OSS.

## What Orbit is NOT for

Two use cases we are deliberately not serving. Each would pull Orbit in a direction that makes it worse for the wedge.

**1. Enterprise surface bolted onto the OSS core.** SOC 2 attestations, SSO/SAML, RBAC, multi-tenant permissions, 24/7 support belong in Orbit Team (the paid hosted product), not the self-hosted OSS binary. The OSS stays focused on the solo wedge; the Team product carries the organizational surface area. Refusing to pollute the OSS is not the same as refusing the demand — we serve the demand through a separate SKU. True Fortune-500 enterprise (FedRAMP, MSAs, dedicated AEs, 12-month sales cycles) is years out and not the near-term Team-product target either.

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

- **Self-hostable OSS tier under a permissive license.** Orbit OSS ships under MIT or Apache 2.0, runs as a single binary, no mandatory cloud dependency. Laptop, container, CI runner, behind a firewall. The OSS is self-sufficient — no single-operator feature is ever gated behind the paid Team product.
- **Open-core split with clean boundaries.** OSS and Team product live in separate repositories with separate licenses. The boundary rule: *"would a solo developer running self-hosted Orbit on their laptop want this?"* — yes → OSS, no → Team. The split is architectural, not a license-flag in shared code. This prevents the boundary from drifting over time and keeps contributors clear on what's open.
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
- Enterprise surface bolted onto the OSS core. Tenancy, SSO, RBAC, audit aggregation belong in Orbit Team, not the self-hosted OSS binary. (Not the same as refusing the demand — the demand is served by the paid SKU.)
- Onboarding designed for non-technical users — patronizing and misaligned with the audience.
- Subscription-arbitrage architectures that assume a single personal CLI account is the backbone. The wedge audience drives multiple agents in parallel against real code; that breaks the per-account assumption.
- Black-box "just trust the agent" patterns. Every agent decision should be inspectable.

## Commercial roadmap: Orbit Team

The wedge today is intent-tracked durable agentic project management for the AI-native solo developer, served by Orbit OSS. The commercial product is **Orbit Team** — hosted, multi-tenant, served to engineering organizations through a champion-led adoption arc. Many agents on many providers running in parallel against shared production code, with cross-agent coordination, team-wide scoreboards, and aggregated audit and tasks across operators.

This is the bet underneath Orbit's architectural choices. Why we over-build audit, scoreboards, multi-provider primitives, per-agent identity attribution, and policy / sandboxing today, even though the wedge audience could get by with less:

- As trust in agents matures, human review shifts from "every diff" to "exception cases." Throughput becomes the bottleneck, and throughput is rate-limited by safe parallel execution. Orbit's fleet primitives are the substrate for that future state.
- The team-adoption arc — solo champion validates → proposes team adoption → company buys Orbit Team — is multi-year, especially for teams whose existing culture isn't yet agent-savvy. The product needs to be ready for that arc before demand materializes.
- BYO-credentials, HTTP/SDK, knowledge-graph-aware, audit-as-feature: these decisions are optimized for the Team product as much as for the solo wedge. They constrain Orbit's near-term ergonomics in ways that pay off only when teams adopt.

Concrete Team-product capabilities (these do NOT live in the OSS):

- **Hosted multi-tenancy.** One Orbit Team instance serving an engineering organization, aggregating audit, tasks, and scoreboards across operators. Closed-source SaaS.
- **Cross-engineer audit aggregation.** Per-operator audit primitives ship in OSS; the cross-operator aggregation, query API, and team-wide audit UI ship in Team.
- **Team-grade fleet metrics.** Cross-engineer agent throughput, team-wide PR merge rates, multi-operator policy enforcement.
- **Organizational governance.** SSO/SAML, RBAC, audit retention policies, compliance attestations.

What lives in OSS, not Team:

- **Per-operator fleet primitives.** Parallel task execution, cross-provider delegation, per-agent scoreboards, per-agent identity in commits — *across many agents on one operator's machine.* This is part of the wedge: the AI-native solo developer drives multiple agents in parallel; Orbit OSS treats fleets as the default execution shape on a single host.

**Commercial GTM motion.** Champion-led, bottom-up: a staff/principal engineer adopts OSS Orbit on personal projects, validates it, becomes the internal advocate, proposes Orbit Team adoption to their organization. Target buyer segment is growth-stage and mid-market companies (10-500 engineers) where champion-led decisions can land without enterprise procurement cycles. Pricing is per-seat hosted; self-serve checkout for small teams, sales-assisted for larger deployments.

The wedge does not depend on the Team product arriving on any specific timeline. But the Team product is what funds the OSS — and what makes full-time work on Orbit sustainable.

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
- Champion-led team adoption fails to convert. Indicator: 12+ months of OSS adoption with zero teams committing to evaluate Orbit Team, no design partners, no inbound from named companies. If the OSS funnel doesn't feed the Team SKU, the open-core thesis is wrong.
- True Fortune-500 enterprise demand (FedRAMP, MSAs, dedicated AEs) materializes faster than the Team product is ready to serve it. Reframe: bring forward enterprise readiness, partner with a systems integrator, or stay focused on growth-stage and mid-market and let the Fortune-500 demand age out.
- Trust in agents matures dramatically faster than expected, collapsing the multi-year arc to months. Reframe the destination as a near-term shape rather than a long-arc one, and update messaging accordingly.
- The team-scale destination itself stops being a coherent niche (e.g., team-scale agent automation collapses into solo tooling above and enterprise platforms below). Revisit the whole document in that case — don't patch around it.
- The hosted Team product launches and customer feedback says the open-core boundary is in the wrong place. Re-locate features across the boundary; do not blur the architectural split between the two repositories.

Until one of those happens, the framing above is the lens.
