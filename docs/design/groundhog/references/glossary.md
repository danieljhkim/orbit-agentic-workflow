# Glossary: Groundhog

Groundhog-specific vocabulary only. Generic terms such as cache TTL, backend, branch, or hunk are excluded unless Groundhog gives them a narrower Orbit-specific meaning. Cross-references point to the numbered Groundhog docs and the mechanism spec so definitions stay tied to the maintained design set.

| Term | Meaning |
|------|---------|
| **Attempt budget** | The retry cap for one checkpoint. It comes from `TaskPlan.checkpoints[*].attempt_budget`; the current runner also applies `attempt_budget_default` at activity level, but today it behaves as a floor rather than a true fallback. See [2_design.md §1](../2_design.md). |
| **Checkpoint** | One structured Groundhog subgoal in the task plan. A checkpoint has `id`, `spec`, typed `success_criteria`, and `attempt_budget`. See [1_overview.md §2](../1_overview.md). |
| **Chronicle** | The current persisted Groundhog artifact at `artifacts.chronicle`. It records checkpoint history using `Day` entries and is the source from which the runner rebuilds successful-checkpoint memory today. See [2_design.md §2](../2_design.md). |
| **Day** | The current implementation's persisted record for one checkpoint execution boundary inside the chronicle. It is an internal Groundhog term that still survives from older drafts even though the higher-level docs talk about checkpoints and attempts. See [2_design.md §2](../2_design.md). |
| **Failure report** | The structured retry payload `{what_tried, what_happened, next_attempt_plan}` emitted by `orbit.groundhog.checkpoint_failure` or synthesized by the runner when an attempt exits without a terminal builtin. See [2_design.md §3](../2_design.md). |
| **Groundhog activity** | The dedicated `ActivityV2Spec::Groundhog` runtime path. It is distinct from `agent_loop` and owns checkpoint parsing, snapshot management, and Groundhog-specific builtins. See [2_design.md §1](../2_design.md). |
| **Groundhog runner state** | The second persisted Groundhog artifact at `groundhog/state.json`. It tracks the active checkpoint, accumulated attempts, latest failure report, and snapshot numbering. See [2_design.md §2](../2_design.md). |
| **Mechanical criterion** | A success criterion that the runtime can verify out of band, such as `command`, `file_exists`, or `file_contains`. See [2_design.md §6](../2_design.md). |
| **Scratch branch** | The git branch `groundhog/<task_id>/day-<n>` created for one Groundhog attempt. It is retained on failure and squash-merged on success. See [2_design.md §4](../2_design.md) and [specs/workspace-snapshot.md](../specs/workspace-snapshot.md). |
| **Structured checkpoint plan** | The typed checkpoint list stored in a task's `plan` field and parsed into `TaskPlan`. This is the authoritative execution structure for Groundhog today. See [1_overview.md §2](../1_overview.md) and [2_design.md §1](../2_design.md). |
| **Success-only memory** | The intended Groundhog direction where later attempts load only prior successful checkpoint summaries plus the active checkpoint's latest failure report. The current implementation approximates this by replaying successful chronicle summaries only. See [2_design.md §3](../2_design.md) and [3_vision.md §1](../3_vision.md). |
| **Verifier boundary** | The runtime step at `checkpoint_success` where Groundhog decides whether a supposedly finished checkpoint really satisfies its mechanical criteria. See [2_design.md §6](../2_design.md). |
