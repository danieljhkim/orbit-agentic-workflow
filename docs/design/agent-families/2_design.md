# Agent Families — Design

**Status:** Draft
**Owner:** human
**Last updated:** 2026-05-16

This document describes the current implementation of Orbit agent families and crew-based role assignment. It covers the family registry, the workspace crew registry, task and CLI override surfaces, and where resolved run metadata is persisted.

## 1. Family Registry

The family registry lives in `crates/orbit-common/src/types/agent_pair.rs`. `all_agent_families()` returns the supported family identifiers, while `agent_from_model()` and `infer_agent_family_from_model()` map model prefixes to those families. Prefix inference remains intentionally conservative because older persisted artifacts may only contain a model string.

Adding a family is still a cross-cutting change: executor assets, sandbox behavior, provider inference, review automation, and scoreboard code all need review. The fixed registry forces that audit instead of silently accepting unknown families.

## 2. Crew Registry

Workspace config now defines concrete role lineups under `[crews.<name>]`. Each crew has three role assignments:

- `planner = { model, provider, backend }`
- `implementer = { model, provider, backend }`
- `reviewer = { model, provider, backend }`

`crates/orbit-core/src/config/raw.rs` owns the TOML shape, and `crates/orbit-core/src/config/runtime.rs` materializes it into `Crew` values from `orbit-common`. Runtime loading rejects incomplete crews and rejects `[workflow].default_crew` when it does not name a defined crew.

The repository default template and `.orbit/config.toml` use `[workflow].default_crew = "opus-codex"` and define at least `opus-codex` and `all-claude`.

## 3. Task and Tool Surface

`Task` has an optional `crew` field. `orbit.task.add` and `orbit.task.update` validate authored crew names against the current workspace registry, and `orbit.task.start` accepts a one-run `crew` override. The runtime re-validates at start time because the config registry can change between task creation and execution.

The precedence chain is:

1. CLI/tool start override `crew`
2. `Task.crew`
3. `[workflow].default_crew`

`orbit.task.show` surfaces the task field and, when the current registry resolves it, the effective crew name plus planner, implementer, and reviewer model strings.

## 4. Run Records

Run-start code resolves the crew before dispatch, emits structured tracing fields for `resolved_crew`, `planner_model`, `implementer_model`, and `reviewer_model`, and persists those four strings on the job run record. Persisting resolved values protects audit trails from later config edits.

Legacy records without crew fields still deserialize because the run-record fields are optional. Display code may use `infer_agent_family_from_model()` only as a recovery path for older artifacts.

## 5. Concerns & Honest Limitations

Crew names are workspace-local strings. Renaming or deleting a crew can break a task that still references the old name, though existing run records keep the resolved model strings.

Duel-plan participant configuration still uses the older family walk and was deliberately left for a separate task. Task-level per-role overrides were also deferred; today a task picks an entire crew, not a single replacement planner or reviewer.

## Task References

- ORB-00042: Onboard Grok (xAI) as a first-class supported agent family.
- ORB-00058: Introduce per-task crew override for agent model selection.

Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
