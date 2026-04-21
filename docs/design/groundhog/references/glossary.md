# Glossary: Groundhog

Lookup table for terms used across the groundhog docs. Sorted alphabetically. Cross-references point at [groundhog_v0.md](../groundhog_v0.md) (vision) and [groundhog_v1.md](../groundhog_v1.md) (first-ship spec).

| Term | Meaning |
|------|---------|
| **Abandonment** | Terminal outcome for a Day when the attempt budget is exhausted without success and no deviation was produced. Recorded as `DayOutcome::Abandoned { reason }`. |
| **Append-only chronicle** | Invariant that the serialized chronicle is a byte-exact prefix of every later serialization. Load-bearing for cache coherence. See [groundhog_v0.md §5.5]. |
| **Attempt** | One pass through a checkpoint within a Day. Carries start/end timestamps, the full tool-call transcript, and an optional `FailureReport`. |
| **Attempt budget** | Maximum number of attempts allowed on a single checkpoint. Default 3. Configurable per checkpoint. |
| **Backend** | The agent runtime binding. Groundhog is defined only for `backend: http`; CLI-backend agents manage their own context and are out of scope. |
| **Breakpoint (cache)** | `cache_control` marker placed at a byte offset in the request prefix. Groundhog places four deterministic breakpoints: system+tools+skills, plan, chronicle, current checkpoint. |
| **Checkpoint** | A pre-defined subgoal in a task plan. Has `id`, `spec`, `success_criteria`, `attempt_budget`. |
| **Checkpoint_deviate** | Verb 4. Payload `{new_checkpoint_spec, rationale}`. Pushes a new checkpoint onto the deviation stack; suspends the current one. |
| **Checkpoint_failure** | Verb 3. Payload `{what_tried, what_happened, next_attempt_plan}`. Closes the Day as failed; workspace reverts; retry within budget. |
| **Checkpoint_success** | Verb 2. Payload `{summary, side_effects}`. Closes the Day as successful; summary appended to chronicle; advance to next checkpoint. |
| **Chronicle** | The growing lineage of completed checkpoint summaries. The agent's long-term memory across Days. |
| **Cognitive entrenchment** | MAR failure mode: an agent reflecting on its own failures reinforces existing assumptions instead of escaping them. Mitigated by the critic (see [groundhog_v0.md §10.7]). |
| **Compaction** | Policy-triggered rewrite of the chronicle prefix when it grows past budget. The only legitimate invalidation of breakpoint 3. |
| **Critic** | Second-perspective agent invoked on attempt 2+ of a checkpoint. Produces a `CriticProposal` proposing a structurally different approach; does not execute tools. |
| **CriticProposal** | `{reuse_prior_plan, alternative_approach, specific_concerns, suggested_tools, escalate}`. |
| **Day** | A single attempt slot for a checkpoint. Begins with fresh agent context and a clean workspace snapshot; ends in success, abandonment, or deviation. |
| **DayOutcome** | Enum: `Success | Abandoned { reason } | DeviatedTo(CheckpointId)`. |
| **Deviation stack** | LIFO list of checkpoints suspended while a pushed checkpoint runs. Max depth 5. |
| **Escalate** | `CriticProposal` field signaling the executor cannot proceed at the current checkpoint scope. Forces a deviation or human review instead of another executor attempt. |
| **Execute** | Verb 1. Normal tool calls progressing the current Day's scratch. |
| **FailureReport** | Structured `{what_tried, what_happened, next_attempt_plan}` written on `checkpoint_failure`. The only attempt content that crosses an attempt boundary. |
| **Four-verb protocol** | Hard constraint on the agent's decision surface: `execute`, `checkpoint_success`, `checkpoint_failure`, `checkpoint_deviate`. |
| **Mechanical criterion** | Success criterion evaluated by the runtime verifier out-of-band from the model loop. Kinds: `command`, `file_exists`, `file_contains`. |
| **Memory** | Persistent state surviving across Days. In Groundhog = the chronicle. Contrasted with scratch. |
| **Planner** | Activity producing the structured checkpoint list before execution begins. Distinct role from the executor. |
| **Rewind** | Restoring workspace state at the end of a failed Day. Git-backed only; non-git side effects do not rewind automatically. |
| **Scratch** | The current Day's working context — full tool-call transcripts. Ephemeral; discarded at end of Day. |
| **Scratch branch** | `groundhog/<task_id>/day-<n>` — git branch holding a Day's uncommitted work. Preferred over `git stash`. |
| **Semantic criterion** | Success criterion judged by the agent at `checkpoint_success` emission. Example: "error message is user-friendly." |
| **Side effect** | `{kind, target, reversible}` record of a mutation performed during a Day. Non-reversible side effects (DB writes, message sends) persist through rewinds. |
| **Success criterion** | Observable outcome required to accept a checkpoint as done. Tagged as mechanical (runtime-verified) or semantic (agent-judged). |
| **TTL (cache)** | Time-to-live for prompt-cache entries. 5m default, 1h for orchestrator-paced flows. |
| **Verifier** | Runtime component executing mechanical criteria in parallel at the verb-2 boundary. Output persists to `artifacts.days[n].verifier_runs[*]`. |
| **Verb** | One of the four protocol actions the agent may emit. See four-verb protocol. |
