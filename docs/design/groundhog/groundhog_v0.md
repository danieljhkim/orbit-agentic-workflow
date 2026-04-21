# Groundhog

**Status:** Draft — pending iteration
**Owner:** TBD
**Last updated:** 2026-04-19

> *"The agent gets to retry each checkpoint like Bill Murray in Groundhog Day — it wakes up fresh but remembers what it learned."*

Groundhog is a structured execution model for coding agents running under Orbit's HTTP agent loop. It replaces unstructured agent improvisation with a plan-driven, checkpoint-granular execution protocol. Failed attempts reset both agent context and workspace state; only distilled summaries survive across attempts. Successful checkpoints accumulate into a chronicle that forms the agent's growing memory.

Groundhog is defined **only for `backend: http`**. CLI-backend agents (Claude Code subprocess) manage their own context and are out of scope.

---

## 1. Motivation

Unstructured agent loops have three recurring failure modes:

1. **Context bloat** — noisy tool-call transcripts crowd out real signal; cache hit rates stay high but attention degrades.
2. **Unbounded improvisation** — agent wanders across scope; emergent decisions are made without plan authority.
3. **Unrecoverable failures** — when an agent fails, partial mutations to the workspace persist, and the next attempt starts from a dirty state with no clean restart.

Groundhog addresses all three by constraining the agent's decision surface and treating each checkpoint attempt as a reset-able "day."

---

## 2. Core Concepts

### 2.1 Checkpoint

A pre-defined subgoal in a task plan. Each checkpoint has:

- **id** — stable identifier
- **spec** — what the agent must accomplish
- **success criteria** — observable outcomes (tests pass, file exists, function defined, etc.)
- **attempt budget** — default 3

### 2.2 Day

A single attempt at a checkpoint. Begins with a fresh agent context and a clean workspace snapshot. Ends in one of three outcomes:

- **success** — agent emitted `checkpoint_success`; summary appended to chronicle; advance
- **failure** — agent emitted `checkpoint_failure`; workspace reverted; retry within budget
- **deviation** — agent emitted `checkpoint_deviate`; new checkpoint pushed onto stack

### 2.3 Chronicle

The growing lineage of completed checkpoint summaries. The chronicle is the agent's long-term memory — everything outside the current day's scratch context.

### 2.4 Memory vs. Scratch

- **Memory (chronicle):** persistent; one entry per completed checkpoint; carried in the cached prompt prefix
- **Scratch (current day's working context):** ephemeral; full tool-call transcripts; discarded at end of day

### 2.5 Four-verb Protocol

The agent's entire decision surface reduces to four verbs:

| Verb | Payload | Effect |
|------|---------|--------|
| `execute` | normal tool calls | progress within current day's scratch |
| `checkpoint_success` | `{summary, side_effects}` | close day; append to chronicle; advance |
| `checkpoint_failure` | `{what_tried, what_happened, next_attempt_plan}` | close day; revert workspace; retry |
| `checkpoint_deviate` | `{new_checkpoint_spec, rationale}` | push new checkpoint; suspend current |

---

## 3. Cache Architecture

Groundhog's structure makes cache breakpoint placement deterministic. No YAML configuration needed.

```
[ system + tools + skills ]          ← breakpoint 1 (fleet-wide)
[ + task plan ]                      ← breakpoint 2 (task-wide)
[ + chronicle so far ]               ← breakpoint 3 (grows per day)
[ + current checkpoint spec ]        ← breakpoint 4 (per-day)
[ working scratch ]                  ← uncached tail
```

**Properties:**
- Breakpoints 1 and 2 are reused across all days of all tasks using the same skill bundle.
- Breakpoint 3 extends by one summary per successful checkpoint.
- Breakpoint 4 is written on day start and read on retries within the same checkpoint.
- Scratch is always uncached; its discard on day-end is free (nothing cached to lose).

**TTL considerations:**
- Use default 5m TTL for active execution.
- Use 1h TTL when the task is known to run across long human gaps (orchestrator-paced or approval-gated flows).

---

## 4. Data Structures

```rust
pub struct Chronicle {
    pub task_id: OrbitId,
    pub plan_id: OrbitId,
    pub days: Vec<Day>,
    pub deviation_stack: Vec<CheckpointId>,  // active pushes, LIFO
}

pub struct Day {
    pub checkpoint_id: CheckpointId,
    pub attempts: Vec<Attempt>,
    pub outcome: DayOutcome,
    pub summary: String,                     // survives to chronicle
    pub side_effects: Vec<SideEffect>,       // what persisted
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
}

pub enum DayOutcome {
    Success,
    Abandoned { reason: String },   // attempt budget exhausted, no deviation produced
    DeviatedTo(CheckpointId),       // agent pushed a new checkpoint
}

pub struct Attempt {
    pub started_at: Timestamp,
    pub ended_at: Timestamp,
    pub tool_calls: Vec<ToolCallRecord>,     // full transcript, for post-hoc review only
    pub failure_report: Option<FailureReport>,
    pub workspace_reverted: bool,
}

pub struct FailureReport {
    pub what_tried: String,
    pub what_happened: String,
    pub next_attempt_plan: String,
}

pub struct SideEffect {
    pub kind: SideEffectKind,       // FileWrite | FileDelete | GitCommit | DbMutation | ...
    pub target: String,             // path, ref, query id, etc.
    pub reversible: bool,
}
```

**Chronicle persistence:** written to task artifact on every day close (success or abandonment). Survives session death.

**Attempt transcripts:** retained in the artifact for post-hoc review but NOT loaded back into context on retries. Only the distilled failure report crosses attempt boundaries.

---

## 5. Rewind Mechanics

Failed attempts must restore workspace state. Each day begins with a snapshot and ends with either merge (success) or revert (failure).

### 5.1 Snapshot

At day start:
- `git stash push -u` on the task branch OR
- `git commit` to a scratch branch `groundhog/<task_id>/day-<n>`

Option 2 is preferred — keeps history inspectable, doesn't conflict with user-level stashes, survives process restart.

### 5.2 Rewind on failure

- `git reset --hard <snapshot_ref>` on the task branch
- Scratch branch retained for post-hoc inspection; garbage-collected after task completes

### 5.3 Commit on success

- Squash the day's work into a single commit on the task branch with message derived from the checkpoint summary
- Drop the scratch branch

### 5.4 Non-git side effects

Rewind is **only guaranteed for git-tracked workspace state.** Other side effects (DB writes, API calls, message sends, file mutations outside workspace) are NOT automatically reverted.

Mitigations:
- `SideEffect::reversible = false` flags irreversible operations
- Summaries must note irreversible effects so retries and deviations can reason about them
- Skills should prefer git-tracked operations over side-channel mutations during execution

### 5.5 Append-Only Chronicle Invariant

**The workspace rewinds. The chronicle does not.**

Anthropic's prompt cache is content-addressed on a byte-exact prefix. Once a `cache_control` breakpoint is placed at byte offset N in a request, any future request that wants to hit that cache entry must produce bytes `[0..N]` byte-identical to the original. Mutating anything below N — including "popping" messages off the conversation tail — invalidates that breakpoint and every breakpoint after it.

Groundhog's logical operations (revert day, pop deviation, abandon checkpoint) all conceptually "remove" content. Implementing them as physical removal would blow breakpoints 3 and 4 on every retry. Instead, the chronicle serializer is **strictly append-only**: logical removal is represented by appending a marker record, never by splicing the message history.

| Logical operation | Naive (cache-hostile) | Cache-safe |
|-------------------|----------------------|------------|
| Day N fails, retry attempt | Splice failed attempt out, re-send | Append `FailureReport`; failed attempt stays below the new tail |
| Day N abandoned | Remove day from chronicle | Append `DayOutcome::Abandoned { reason }` marker |
| Pop deviation (sub-plan resolved) | Splice sub-plan messages out | Append "deviation resolved" marker; sub-plan messages remain |
| Workspace rewind on failure | — | Out-of-band — touches git, not messages. Safe by construction. |

**Serializer contract.** The chronicle serializer must satisfy:

```
for all n < m:
    Chronicle::serialize_at(day_n) == Chronicle::serialize_at(day_m)[..len_at(day_n)]
```

That is, each serialized chronicle is a byte-exact prefix of every later serialization of the same chronicle. This is a load-bearing property of the cache architecture, not an optimization. A unit test on the serializer should assert it directly.

**The one legitimate invalidation: compaction.** If the chronicle grows past a budget (e.g. 50+ successful days), days 1..k get compressed into a summary and breakpoint 3's prefix is rewritten. This is an explicit, observable event — one cache-write cost, then amortized across all subsequent requests. Compaction is runtime-triggered on policy, never agent-triggered mid-attempt.

**What this rules out.** Features that would require mid-run mutation of the prefix are off-limits under the current cache architecture:

- "Edit the failure report of attempt 1 based on what we learned in attempt 2" — would rewrite prior bytes. Instead: append a superseding report.
- "Drop the noisy transcript of day 3 after it succeeded" — would rewrite prior bytes. Instead: redact at compaction time only.
- "Reorder days" — never. The chronicle is a log, not an editable list.

If a future feature genuinely needs mid-run mutation, it is a compaction-class operation and must pay the cache-write cost explicitly.

---

## 6. Deviation Stack

When the plan is wrong, incomplete, or encounters unforeseen dependencies, the agent pushes a new checkpoint onto a stack.

### 6.1 Mechanics

- `checkpoint_deviate(new_spec, rationale)` pushes onto `Chronicle.deviation_stack`
- Current checkpoint is suspended; executor begins the pushed checkpoint as a new Day
- On success of the pushed checkpoint: pop; resume suspended checkpoint with a new Day (fresh attempt)
- On abandonment of the pushed checkpoint: pop; mark suspended checkpoint as blocked

### 6.2 Stack discipline

- LIFO only — no jumping across the stack
- No branching (if you need to try two alternatives, that's two sequential deviations)
- Max stack depth: proposed **5** (prevents runaway deviation chains; blocks task if exceeded)

### 6.3 Deviation telemetry

Every deviation is a first-class event in the chronicle with `{from_checkpoint, new_spec, rationale, pushed_at}`. Surfaced in scoreboards as a signal of planner quality.

---

## 7. Retry Budget

- Default: 3 attempts per checkpoint
- Configurable per checkpoint in the plan
- On budget exhaustion: force a deviation (`investigate_<checkpoint_id>` with rationale `"attempts exhausted"`) OR mark the task blocked if deviation depth is at max

---

## 8. Orbit Integration

### 8.1 Activity shape

One new activity kind (or a flag on `agent_loop`):

```yaml
kind: groundhog
backend: http
provider: claude
wall_clock_timeout_seconds: 3600
plan_source: "{{ input.task_id }}"       # reads checkpoints from task plan
attempt_budget_default: 3
deviation_depth_max: 5
```

### 8.2 Plan schema additions

Task plan gains structured `checkpoints`:

```yaml
plan:
  checkpoints:
    - id: ckpt_01
      spec: "Update the `foo` function to handle null inputs."
      success_criteria:
        - "File crates/foo/src/lib.rs modified"
        - "make build passes"
      attempt_budget: 3
    - id: ckpt_02
      spec: "Wire the new behavior into the caller."
      success_criteria:
        - "Callsite in crates/bar/src/use_foo.rs uses the new signature"
      attempt_budget: 2
```

### 8.3 Planner role

A dedicated planner activity produces the checkpoint list before execution starts. Groundhog-style execution presupposes a structured plan; unstructured tasks either get routed to a traditional `agent_loop` or start with a planning checkpoint whose output is more checkpoints.

### 8.4 Tools

- `orbit.groundhog.checkpoint_success` — verb 2
- `orbit.groundhog.checkpoint_failure` — verb 3
- `orbit.groundhog.checkpoint_deviate` — verb 4
- `orbit.groundhog.side_effect` — log a side effect during execution (feeds `Day.side_effects`)
- `orbit.groundhog.chronicle` — read-only view of the chronicle for debugging

### 8.5 State persistence

- Chronicle: persisted to task artifact under `artifacts.chronicle` on every day close
- Scratch transcripts: persisted to `artifacts.days[n].attempts[m].tool_calls` (review-only)
- Active deviation stack: persisted to `artifacts.chronicle.deviation_stack`

### 8.6 Critic agent (retry-only)

To mitigate cognitive entrenchment (§10.7), retries are mediated by a second agent — the **critic** — whose role is to propose a structurally different approach after a failure. The critic does not execute; it produces a proposal that the executor uses as input for the next attempt.

**When it fires:** only on attempt 2+ of a checkpoint. Attempt 1 runs without the critic to avoid paying its cost on checkpoints that succeed first try.

**Inputs:**
- Checkpoint spec
- Chronicle summaries up to this checkpoint (not full transcripts)
- Failed attempt's `FailureReport` (`what_tried`, `what_happened`, executor's `next_attempt_plan`)
- Optional: executor's full scratch transcript from the failed attempt (enables deeper critique; costs more tokens — tunable)

**Outputs:**
```rust
struct CriticProposal {
    reuse_prior_plan: bool,           // rare; usually false
    alternative_approach: String,     // structurally different direction
    specific_concerns: Vec<String>,   // e.g. "executor keeps editing file X but root cause is in file Y"
    suggested_tools: Vec<String>,     // tools to try that weren't used
    escalate: Option<EscalateReason>, // critic signals executor cannot proceed
}
```

**Escalation path:** if the critic emits `escalate: Some(...)`, the runtime skips the executor retry and forces a deviation (or human review, depending on configuration). The critic can signal "this isn't solvable at the current checkpoint scope" and bypass wasted retries.

**Activity shape:**

```yaml
kind: groundhog_critic
backend: http
provider: claude
# cheaper model acceptable; critic is analysis, not execution
model: "claude-haiku-4.5"
wall_clock_timeout_seconds: 300
```

The critic may use a cheaper / smaller model than the executor. The job is structured analysis of a failure report, not tool-using execution — a smaller model typically suffices and the cost asymmetry matters when retries are frequent.

**Runtime flow for a retry:**

```
attempt_n failed → FailureReport written to artifact
  ↓
invoke critic activity (§8.6) with {spec, chronicle_summaries, failure_report}
  ↓
critic returns CriticProposal
  ↓
  ├─ if escalate: runtime forces deviation; skip executor retry
  └─ else: invoke executor for attempt_n+1 with failure_report + CriticProposal in context
```

**Guarding against critic entrenchment.** If consecutive critic proposals across retries are themselves structurally similar (measured by embedding distance or simple keyword overlap), the runtime escalates to forced deviation — belt-and-suspenders per §10.7 fallback.

**Observability:** every critic invocation is a first-class event in the Day's attempts log, including the proposal, whether it escalated, and whether the subsequent executor attempt succeeded. Feeds scoreboard signal: *critic proposal → success rate*. Low values mean the critic isn't actually helping; high values validate the design.

### 8.7 Success-criteria verification

Success criteria on a checkpoint are tagged as either **mechanical** or **semantic**.

| Kind | Example | Evaluated by |
|------|---------|--------------|
| Mechanical | `command`, `file_exists`, `file_contains` | Runtime verifier, out-of-band from the model loop |
| Semantic | "error message is user-friendly" | Agent judgment at `checkpoint_success` emission |

**Flow on `checkpoint_success`:**

1. Runtime reads the checkpoint's mechanical criteria from the plan.
2. Runtime runs them in parallel against the workspace.
3. On any failure: synthesize a `FailureReport` naming the failing criterion + captured output (truncated), and treat the day as `checkpoint_failure` — retry per §7.
4. On all-pass: day closes as success; summary appended to chronicle.

**Consequences:**

- Build/test output never enters the scratch transcript on success. Zero cache pressure from passing runs.
- Independent mechanical checks parallelize automatically from the runtime, not sequentially through the agent's turn loop.
- Full verifier output persists to `artifacts.days[n].verifier_runs[*]` for post-hoc debugging.

**In-flight diagnostics remain in-agent.** If the agent wants to run `cargo check` *during* a day to diagnose a type error mid-edit, it invokes the build tool directly. Only gate-checks at verb-2 boundary route through the verifier. The distinction is enforced by convention; the tool surface does not prevent mid-day builds.

**Criterion schema:**

```yaml
success_criteria:
  - kind: command
    command: "cargo test -p orbit-engine"
    expect_exit: 0
  - kind: file_exists
    path: "crates/foo/src/lib.rs"
  - kind: file_contains
    path: "crates/foo/src/lib.rs"
    pattern: "fn handle_null"
  - kind: semantic
    statement: "Error messages are user-friendly (agent-judged)"
```

---

## 9. Observability

### 9.1 Per-task metrics

- Total days run (= checkpoints + deviations)
- Deviation count (signal: high = weak plan)
- Abandonment count (signal: high = executor can't cope OR plan is impossible)
- Average attempts per checkpoint
- Workspace revert count

### 9.2 Scoreboard signals

- **Planner agent quality:** deviations per task authored
- **Executor agent quality:** success rate per checkpoint, average attempts
- **Plan quality by task type:** which task classes are Groundhog-suitable vs. not

### 9.3 Debug surface

`orbit tool run orbit.groundhog.chronicle --input '{"task_id": "..."}'` renders:

```
Task T20260419-1234 (chronicle)
├─ Day 1: ckpt_01 (success, 2 attempts, 4m)
├─ Day 2: ckpt_02 (deviated → ckpt_02a)
├─ Day 3: ckpt_02a (success, 1 attempt, 2m) ← pushed
├─ Day 4: ckpt_02 (success, 1 attempt, 3m)  ← resumed
└─ Day 5: ckpt_03 (abandoned, 3 attempts)
```

---

## 10. Concerns & Honest Limitations

### 10.1 Plan quality is the new bottleneck

System output quality = plan quality. A vague or wrong plan produces thrashing executors. This shifts investment from "make the agent smarter" to "make the planner better," which is a real organizational change.

### 10.2 Not all tasks are Groundhog-suitable

Exploratory debugging, research spikes, and "figure out why X is broken" tasks cannot be checkpoint-planned upfront. Groundhog should not be forced onto these. Task router must detect mismatch (by task type, by planner self-assessment, or by falling back after N deviations) and route to traditional agent mode.

### 10.3 Summary quality is load-bearing

Between attempts, only the failure report survives. Between checkpoints, only the summary survives. Poor summaries compound: later checkpoints lose critical context. Enforcing structured summary formats helps but doesn't eliminate the risk.

### 10.4 Non-git side effects are not auto-reverted

Rewind only covers git-tracked state. Database mutations, API calls, message sends persist. Skills must either avoid these during execution or flag them explicitly. This is a sharp edge that will cut someone eventually.

### 10.5 Emergent insight has one channel

Agent observations that don't fit the current checkpoint must be pushed as deviations. This is a simplification of a rich problem — some observations are worth noting but not worth deviating over. Open question in §12.

### 10.6 The protocol shape won't survive first contact

The four-verb protocol is a hypothesis. Real executors may struggle to classify their situation into one of four verbs, or may over-use deviation when simple execution would suffice. Expect the protocol to evolve based on end-to-end prototype data.

### 10.7 Cognitive entrenchment (MAR, Dec 2025)

Multi-Agent Reflexion (arXiv 2512.20845) identifies a specific failure mode in single-agent reflection systems: an agent reflecting on its own failures tends to reinforce existing assumptions, producing retries that are structurally similar to failed attempts. Reflection does not escape local optima; it reinforces them.

This is **directly relevant to Groundhog**: the `checkpoint_failure` → `retry` loop assumes the agent can self-diagnose and propose a meaningfully different `next_attempt_plan`. If MAR's finding holds, this assumption is weak.

**Chosen mitigation: Critic-on-retry.** Retries include a second-perspective agent whose job is to propose a different approach, not execute. The critic only runs on retries (attempt 2+), so the cost is bounded and proportional to how often the executor fails. Cheaper than full MAR since the critic does not run every turn. See §8.6 for the activity shape.

Fallback mitigations retained as complements, not replacements:

- **Externally forced deviation.** After N attempts where the critic's proposals are themselves structurally similar (entrenchment two layers deep), the runtime forces a deviation or escalates to human review. Belt-and-suspenders for critic degeneration.
- **Plan-level intervention.** If an executor hits entrenchment on checkpoint X and the critic cannot unstick it, the planner (different agent) is invoked to revise the plan rather than the executor retrying further.

This concern was previously hand-waved as "retry budget prevents infinite thrash." The budget prevents infinite *time*, not infinite *structural similarity*. Distinct problem; the critic is the primary response.

---

## 11. Open Questions

1. **Planner vs. executor as separate agents:** one agent or two? If two, handoff contract needs to be defined. If one, agent switches modes — possibly confusing.
2. **Plan authored by human or agent?** Both? If agent-authored, what activity produces plans, and how is plan quality reviewed?
3. **Who confirms success criteria?** The agent self-reports success — do we trust it? Or does a separate verifier activity gate the `checkpoint_success` verb?
4. **Deviation budget:** max stack depth of 5 is a guess. Needs empirical calibration.
5. **Chronicle compaction:** after 50 successful checkpoints, the chronicle might be too large. Do we summarize chronicle-of-chronicle at some threshold?
6. **Cross-task chronicle sharing:** if task A's chronicle is relevant to task B (same area of code), can B start with A's chronicle as warm context? Probably yes, probably a follow-up feature.
7. **Emergent-insight channel:** should there be a fifth verb for "noticed something, continuing" that appends to the chronicle without deviating?
8. **Integration with knowledge graph:** does success criteria validation use `orbit-knowledge` lookups? Could make verification much stronger.
9. **Branch naming collision:** `groundhog/<task_id>/day-<n>` — any conflicts with existing branching conventions? Users with their own `groundhog/` namespace?
10. **What happens on session crash mid-day?** Chronicle is persisted; scratch is lost. Recovery: resume the checkpoint from scratch or abandon the day and start over?

---

## 12. Proposed Task Breakdown

Sequencing roughly bottom-up. Some can go in parallel.

| Task | Scope | Dependencies |
|------|-------|--------------|
| G1 | `Chronicle`, `Day`, `Attempt` types + persistence shape | — |
| G2 | Plan schema additions (`checkpoints[]` in task plan) | — |
| G3 | Four-verb protocol + tool definitions | G1 |
| G4 | Workspace snapshot/rewind mechanism (git-based) | — |
| G-V | Runtime verifier for mechanical success criteria (§8.7) | G1, G2 |
| G5 | Groundhog activity runner (orchestrates days within a task) | G1, G2, G3, G4, G-V |
| G6 | Cache breakpoint placement integrated with chronicle | G5 |
| G7 | Observability tool (`orbit.groundhog.chronicle`) + scoreboard signals | G1, G5 |
| G8 | Task router: Groundhog-suitable detection / fallback | G5 |
| G9 | Planner activity producing structured plans | G2 |
| G10 | Critic activity (§8.6): `groundhog_critic` kind + `CriticProposal` schema | G1, G3 |
| G11 | Retry orchestration: invoke critic between attempt N and N+1; wire escalation | G5, G10 |
| G12 | Critic entrenchment detection (proposal similarity check + forced deviation) | G10, G11 |

G1, G2, G4 can proceed in parallel as foundations. G10 can start once G1 and G3 land. G11 and G12 are the wiring layer — they land after G5 and G10.

---

## 13. Prior Work

Groundhog is **opinionated synthesis of well-studied patterns**, not a novel invention. The components below exist in the 2025–2026 literature and in production frameworks; Groundhog's contribution is the specific combination, the discipline of the four-verb protocol, and the coupling to prompt-cache architecture.

### 13.1 Checkpointing

- **LangGraph** — `StateGraph` with `PostgresSaver` for durable checkpoints; de facto standard for Python agent graphs.
- **Microsoft Agent Framework** — checkpoints as a first-class workflow concept.
- **Temporal-style activity checkpointing** — deterministic replay against event history; widely referenced as the mental model.

Groundhog's `Day` is a checkpoint unit; nothing novel here.

### 13.2 Git-based rollback

- **AgentGit** (arXiv 2511.00628, Nov 2025) — Git-like rollback and branching for multi-agent systems.
- **Plandex** — cumulative diff review sandbox, branches per plan update, rollback and debug commands.
- **Aider** — git-native `/undo` for agent-made changes.

Groundhog's workspace snapshot/rewind (§5) is a narrower instance of this pattern, scoped to single-executor sessions.

### 13.3 Context as version-controlled workspace

- **Git Context Controller (GCC)** (arXiv 2508.00031, Aug 2025) — Agent context with explicit COMMIT/BRANCH/MERGE/CONTEXT operations, milestone-based checkpointing, hierarchical retrieval. This is the most direct analog to Groundhog's chronicle; GCC is more general (arbitrary branching), Groundhog is a deliberate simplification (LIFO-only deviations).

### 13.4 Retry with reflection

- **Reflexion** (Shinn et al., arXiv 2303.11366, 2023) — Verbal self-reflection appended to context on retries; 91% pass@1 on HumanEval.
- **Multi-Agent Reflexion (MAR)** (arXiv 2512.20845, Dec 2025) — Identifies *cognitive entrenchment* in single-agent reflection: an agent reflecting on its own failures tends to reinforce existing assumptions and repeat structurally similar mistakes. MAR's answer is multiple agents reflecting from different perspectives.

Groundhog's `checkpoint_failure` → retry loop is a Reflexion-style mechanism. MAR's finding is **directly relevant as a known failure mode** — see §10.7.

### 13.5 Plan-driven coding agents

- **Plandex** — terminal-based, 2M token context, plan branches, cumulative diff review. The nearest existing OSS coding agent in shape.

Groundhog diverges from Plandex primarily in autonomy: Plandex keeps human-in-the-loop per subtask; Groundhog is autonomous between human-visible checkpoints.

### 13.6 What may still be distinctive in Groundhog

Softened claims after literature review:

- **Chronicle coupled to prompt-cache breakpoints.** The specific mapping of task-structure layers to `cache_control` markers for Anthropic-style prompt caches does not appear in the literature I've surveyed. Possibly novel, possibly just unpublished-because-infrastructure.
- **Four-verb protocol as a hard constraint on agent decision surface.** More restrictive than GCC's operation set; more opinionated than LangGraph's StateGraph. Whether the constraint holds up in practice is the key open empirical question.
- **LIFO-only deviation.** A deliberate simplification of branching systems (GCC, AgentGit). Simpler to debug; loses the ability to compare alternative branches.

None of these rise to "novel primitive." Treat Groundhog as a productization synthesis with opinionated defaults, not a research contribution.

---

## 14. References

### Conversation lineage
- Conversation thread that produced this design: orbit session 2026-04-19

### Cited prior work
- Shinn, N. et al. *Reflexion: Language Agents with Verbal Reinforcement Learning.* arXiv:2303.11366 (2023).
- *Git Context Controller: Manage the Context of LLM-based Agents like Git.* arXiv:2508.00031 (Aug 2025).
- *AgentGit: A Version Control Framework for Reliable and Scalable LLM-Powered Multi-Agent Systems.* arXiv:2511.00628 (Nov 2025).
- *MAR: Multi-Agent Reflexion Improves Reasoning Abilities in LLMs.* arXiv:2512.20845 (Dec 2025).
- LangGraph StateGraph documentation.
- Microsoft Agent Framework — Workflows → Checkpoints.
- Plandex — https://github.com/plandex-ai/plandex

### Orbit-internal
- [docs/design/activity-job.md](./activity-job.md) (pending rename from `docs/wiki/v2_plan/activity-job-v2.md`)
- [POSITIONING.md](../POSITIONING.md) — audit-as-product, fleet primitives lens
