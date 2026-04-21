# Groundhog — Decisions

ADR-style log of non-obvious design choices behind Groundhog. Each entry names the decision, the context that forced it, what we chose, and what we traded away. Entries are append-only and keyed by number; superseded entries are marked, not deleted.

Format for each entry: **Status · Date**, then *Context → Decision → Consequences*. Sources are [groundhog_v0.md](./groundhog_v0.md) (vision) and [groundhog_v1.md](./groundhog_v1.md) (first-ship spec).

Groundhog is pre-implementation; every entry below is **Proposed** until the relevant Orbit task lands. When a task ships, replace `Proposed` with `Accepted` and cite the task ID.

---

## ADR-001 — Four-verb protocol as a hard constraint

**Status:** Proposed · 2026-04

**Context.** Unstructured agent loops wander across scope and blur the line between "making progress" and "done." Retry logic, deviation tracking, and success reporting get implemented ad-hoc inside each activity.

**Decision.** Reduce the agent's decision surface to exactly four verbs: `execute`, `checkpoint_success`, `checkpoint_failure`, `checkpoint_deviate`. Every other decision the agent might make is absorbed into one of these shapes. See [groundhog_v0.md §2.5].

**Consequences.**
- Runtime can make deterministic choices (close Day, revert workspace, push deviation) without parsing freeform text.
- Telemetry and scoreboard signals collapse onto a small fixed surface.
- Cost: real executors may struggle to classify ambiguous situations into four verbs, or over-use deviation when simple execution would suffice. Explicit risk in [groundhog_v0.md §10.6].

---

## ADR-002 — Defined only for `backend: http`, not `backend: cli`

**Status:** Proposed · 2026-04

**Context.** CLI-backend agents (Claude Code subprocess) manage their own context. Injecting Groundhog's chronicle and breakpoints into a process we do not control is undefined.

**Decision.** Scope Groundhog to `backend: http` only. CLI-backend tasks route to the traditional `agent_loop` activity.

**Consequences.**
- HTTP control over prompt prefix and cache breakpoints is a prerequisite; we keep that as a design invariant.
- CLI-backend tasks get no Groundhog benefits today. Retained as a non-goal in [groundhog_v1.md §2].
- Open whether CLI ever re-enters scope. If the CLI backend gains prompt-context hooks, we revisit.

---

## ADR-003 — Append-only chronicle, never splice

**Status:** Proposed · 2026-04

**Context.** Anthropic's prompt cache is content-addressed on a byte-exact prefix. Mutating anything below a breakpoint invalidates that breakpoint and every breakpoint after it. Logical operations like "revert a failed Day" or "pop a resolved deviation" naively imply splicing the message history — which would blow breakpoints 3 and 4 on every retry.

**Decision.** Serializer is strictly append-only. Logical removal is represented by appending a marker record, never by editing prior bytes. Contract: `Chronicle::serialize_at(day_n)` must be a byte-exact prefix of `Chronicle::serialize_at(day_m)` for all `n < m`.

**Consequences.**
- Prompt-cache hit rate stays high across retries.
- Workspace rewinds remain safe because they touch git, not messages.
- Cost: the chronicle contains records of things that logically "didn't happen" (failed attempts, popped deviations). Reader-side rendering has to filter, which is acceptable.
- Rules out mid-run mutations like "edit the attempt-1 failure report based on attempt-2 learnings" — those become superseding records instead.

---

## ADR-004 — Git scratch branch for workspace rewind, not `git stash`

**Status:** Proposed · 2026-04

**Context.** Failed attempts must restore workspace state. Two candidates: `git stash push -u` on the task branch, or a commit to a scratch branch `groundhog/<task_id>/day-<n>`.

**Decision.** Scratch branch. The stash approach conflicts with user-level stashes, doesn't survive process restart, and hides history from post-hoc inspection. See [groundhog_v0.md §5.1].

**Consequences.**
- Failed attempts leave inspectable branches; GC after task completion.
- Safe across process restart.
- Cost: branch proliferation during a long task. GC is a runtime responsibility.

---

## ADR-005 — LIFO-only deviation, no arbitrary branching

**Status:** Proposed · 2026-04

**Context.** When the plan is wrong, the agent needs to push a new subgoal. General branching (like Git Context Controller) is expressive but complicates chronicle structure, cache coherence, and debugging.

**Decision.** Deviations are a LIFO stack with max depth 5. No jumping across the stack. "Try two alternatives" is two sequential deviations, not two branches. See [groundhog_v0.md §6].

**Consequences.**
- Chronicle rendering stays linear — a log, not a tree.
- Loss of the ability to compare alternative branches side-by-side.
- Simpler to debug and reason about; scoreboard signal (deviation count per task) is meaningful without branch-aggregation logic.
- Max depth 5 is a guess pending empirical calibration.

---

## ADR-006 — Critic-on-retry instead of full Multi-Agent Reflexion

**Status:** Proposed · 2026-04

**Context.** MAR (arXiv 2512.20845) identifies cognitive entrenchment in single-agent reflection: an agent reflecting on its own failures reinforces existing assumptions. The `checkpoint_failure → retry` loop assumes self-diagnosis produces meaningfully different next attempts. That assumption is weak.

**Decision.** Introduce a second "critic" agent on retries only (attempt 2+). Critic analyzes the failure report and proposes a structurally different approach but does not execute. See [groundhog_v0.md §8.6 and §10.7].

**Consequences.**
- Entrenchment mitigation without paying MAR's per-turn cost.
- Critic can escalate to forced deviation when the checkpoint isn't solvable at its current scope.
- Cost: one extra model call per retry. Belt-and-suspenders fallback: if critic proposals themselves trend structurally similar, force deviation anyway.

---

## ADR-007 — Critic uses a cheaper model than the executor

**Status:** Proposed · 2026-04

**Context.** Critic work is structured analysis of a failure report — not tool-using execution. Smaller models typically suffice for this shape of task, and retries are frequent enough that cost asymmetry matters.

**Decision.** Default the critic activity to a cheaper / smaller model (e.g. `claude-haiku-4.5`). Tunable per deployment.

**Consequences.**
- Budget impact of adding the critic stays bounded.
- Risk: cheaper model produces weak proposals, entrenchment returns. Observability signal (*critic proposal → success rate*) catches this.

---

## ADR-008 — Mechanical success criteria verified out-of-band

**Status:** Proposed · 2026-04

**Context.** Letting the agent judge success has two failure modes: false-positive success claims, and build/test output bloating scratch context. Running verification through the agent's turn loop magnifies both.

**Decision.** Tag success criteria as mechanical or semantic. Mechanical criteria (`command`, `file_exists`, `file_contains`) run in parallel from the runtime verifier at the `checkpoint_success` boundary. Semantic criteria remain agent-judged. See [groundhog_v0.md §8.7].

**Consequences.**
- Build/test output never enters scratch on a passing run — zero cache pressure.
- Mechanical checks parallelize automatically.
- Full verifier output persists to `artifacts.days[n].verifier_runs[*]` for post-hoc debugging.
- In-flight diagnostic builds the agent runs mid-Day are out-of-band from the verifier — convention, not enforced by the tool surface.

---

## ADR-009 — Attempt budget default of 3

**Status:** Proposed · 2026-04

**Context.** Retries are expensive (wall clock, tokens, rewinds) and provide diminishing returns past a small constant. The budget needs to be small enough to keep failed checkpoints from dominating a task's cost, large enough that occasional flaky failures don't force deviation.

**Decision.** Default `attempt_budget: 3` per checkpoint. Configurable in the plan. On exhaustion, force a deviation tagged "attempts exhausted" or mark the task blocked when deviation depth is at max.

**Consequences.**
- Worst-case cost per checkpoint is bounded at 3× executor + 2× critic.
- Genuinely hard checkpoints either produce a useful deviation proposal or surface as blocked — both are actionable signals.
- Number is a guess; calibrate after real-run telemetry.

---

## ADR-010 — Chronicle persisted per Day close, transcripts retained but not replayed

**Status:** Proposed · 2026-04

**Context.** Sessions crash. Chronicle loss would force full task restart. Transcripts are large and noisy — replaying them into retry context would reintroduce the very noise Groundhog exists to suppress.

**Decision.** Persist the chronicle to task artifact on every Day close (success or abandonment). Retain full attempt transcripts in the artifact for post-hoc review, but never load them back into agent context on retries. Only the distilled `FailureReport` crosses attempt boundaries.

**Consequences.**
- Session crash recovery preserves long-term memory.
- Post-hoc inspection (debugging, scoreboard analysis) has full fidelity.
- Retry context stays small and stable — cache coherence holds.
- Cost: artifact size grows with task length. Compaction is the escape hatch (see [groundhog_v0.md §5.5]) but is explicit and policy-gated.
