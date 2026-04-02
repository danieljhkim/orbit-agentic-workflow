# Orbit: Local-First Agentic Workflow Engine

Orbit is a lightweight, local-first execution engine designed for agent-driven software delivery. It provides the core primitives necessary to orchestrate complex, multi-step AI workflows directly within a development environment.

Orbit runs directly on agent CLIs - no API keys required.

---

## Quick Start

**Prerequisites**: Rust + Codex CLI + Claude Code (subscriptions required)

Example flow:

```bash
# install orbit binary
make install

# Initialize `~/.orbit` directory with default global configurations, such as skills, activities, jobs, and config.toml.
orbit init

# cd into your repo 
cd <repo>

# Initialize orbit workspace - creates `.orbit` dir in the repo_root, and tasks, diagnostics, and scoreboard directories.
orbit workspace init

# Prompt an agent to create tasks (they will know what to do via orbit skills):
"Create an orbit task for <task_description>"
"Create another orbit task for <another_task_description>"

# once tasks are created, review the tasks
orbit task show <task_id> # or directly check <repo_root>/.orbit/tasks/proposed directory

# approve the tasks if they are valid
orbit task approve <task_id> --note "LGTM"

# run the parallel task pipeline (this will dispatch tasks to the parallel task worker, review the tasks, and merge the PR)
orbit job run job_parallel_task_pipeline
```

---

## Overview

### Orbit Artifacts

Orbit artifacts have 2 scopes:
- **global scope**: Initialized via `orbit init`. Global artifacts are scoped to the global root directory (e.g. ~/.orbit/)
- **workspace scope**: Initialized via `orbit workspace init`. Workspace artifacts override global scopes, and is located in a workspace (e.g. <workspace_root>/.orbit/)

Orbit operates through a structured filesystem hierarchy under `.orbit/`:

```
.orbit/
├── activities/       # Atomic units of work (YAML)
├── diagnostics/      # Runtime diagnostics and health checks (workspace scoped only)
├── jobs/
│   ├── jobs/         # Job definitions — ordered chains of activities
│   └── runs/         # Immutable execution audit logs per job run (workspace scoped only)
├── scoreboard/       # Agent performance tracking (PR merge rates, friction bounty) (workspace scoped only)
├── skills/           # Markdown-based skill instructions loaded by agents
└── tasks/            # Task artifacts organized by lifecycle state (workspace scoped only)
```

### Tasks

Tasks are work items for agent/human coordination and project tracking - you can think of them as jira tickets. 

Orbit jobs are tightly coupled with orbit tasks. If you are working with other issue tracking tools like Linear or Jira, simply have agents create complementary orbit tasks.

- **Work unit for execution**: Represents a discrete piece of work (feature, bug fix, chore, refactor)
- **Lifecycle states**: Proposed → Backlog → InProgress → Review → Done (with branching to Blocked, Archived, Rejected, Someday)
- **Execution tracking**:
    - execution_summary for recording what was actually done
    - Branch and PR number for code changes
    - Comments from team members
    - History log of all transitions and updates

### Activities

Activities are atomic, reusable units of work:

- Self-contained operations with defined schemas (input/output)
- Each has a spec_type that determines how it's implemented (i.e. automation, agent_invoke, cli_command)
- Can be run individually via CLI or as part of a job
- Example activities: run_tests, review_batch_pr, review_tasks, implement_change

### Jobs

You can chain one or more **activities** and run them as a single job.
- Jobs are workflows/pipelines that orchestrate activities. 
- Designed for automation/repeatability

### Default Jobs

Orbit ships with six default jobs that cover the full task lifecycle — from batch dispatch through implementation, review, and merge.

#### `job_parallel_task_pipeline`

The main entry point. Selects a conflict-free batch of backlog tasks, dispatches parallel workers to implement them in a shared worktree, verifies the result, commits, opens a PR, and hands off to the review cycle.

```bash
orbit job run job_parallel_task_pipeline
# with custom input:
orbit job run job_parallel_task_pipeline --input '{"base": "agent-main", "parallelism": 3}'
```

<details>
<summary>Job YAML</summary>

```yaml
schemaVersion: 1
job:
  job_id: job_parallel_task_pipeline
  state: enabled
  max_active_runs: 1
  default_input:
    base: agent-main
    parallelism: 3
  steps:
    - target_type: activity
      target_id: dispatch_and_plan_batch
      agent_cli: claude
      model: opus
      condition: always
      timeout_seconds: 2000

    - target_type: activity
      target_id: snapshot_batch_state
      condition: on_success
      timeout_seconds: 30

    - target_type: activity
      target_id: parallel_dispatch_tasks
      condition: on_success
      timeout_seconds: 7200

    - target_type: activity
      target_id: verify_batch
      condition: on_success
      timeout_seconds: 600

    - target_type: activity
      target_id: commit_and_open_batch_pr
      condition: on_success
      timeout_seconds: 300

    - target_type: job
      target_id: job_batch_review_cycle
      condition: on_success
      timeout_seconds: 7200
```
</details>

#### `job_local_task_pipeline`

A lightweight local-only pipeline. Plans, implements, and commits without requiring `gh` or GitHub access. Changes are committed directly to the base branch — no PR, review cycle, or merge step. Review diffs locally and push when ready.

```bash
orbit job run job_local_task_pipeline
# with custom input:
orbit job run job_local_task_pipeline --input '{"base": "agent-main", "parallelism": 3}'
```

<details>
<summary>Job YAML</summary>

```yaml
schemaVersion: 1
job:
  job_id: job_local_task_pipeline
  state: enabled
  max_active_runs: 1
  default_input:
    base: agent-main
    parallelism: 4
  steps:
    - target_type: activity
      target_id: dispatch_and_plan_batch
      agent_cli: claude
      model: opus
      timeout_seconds: 2400

    - target_type: activity
      target_id: snapshot_batch_state
      condition: on_success
      timeout_seconds: 15

    - target_type: activity
      target_id: parallel_dispatch_tasks
      condition: on_success
      timeout_seconds: 7200

    - target_type: activity
      target_id: finalize_tasks
      agent_cli: claude
      model: sonnet
      condition: always
      timeout_seconds: 3600

    - target_type: activity
      target_id: commit_batch_changes
      condition: on_success
      timeout_seconds: 300
```
</details>

#### `job_parallel_task_worker`

Implements a single task in the shared worktree. Spawned automatically by `job_parallel_task_pipeline` — one worker per task, up to 16 concurrent.

```bash
# typically invoked by the pipeline, but can be run directly:
orbit job run job_parallel_task_worker --input '{"task_id": "T20260401-0001", "base": "agent-main"}'
```

<details>
<summary>Job YAML</summary>

```yaml
schemaVersion: 1
job:
  job_id: job_parallel_task_worker
  state: enabled
  max_active_runs: 16
  default_input:
    base: agent-main
    verification_mode: deferred
  steps:
    - target_type: activity
      target_id: implement_change
      agent_cli: codex
      model: gpt-5.4
      condition: on_success
      timeout_seconds: 4000

    - target_type: activity
      target_id: update_task
      condition: on_success
      timeout_seconds: 15
```
</details>

#### `job_batch_review_cycle`

Reviews a batch PR against all tasks' acceptance criteria, syncs review threads to GitHub, and either merges on approval or enters the fix loop.

```bash
orbit job run job_batch_review_cycle --input '{"base": "agent-main", "pr_number": "42"}'
```

<details>
<summary>Job YAML</summary>

```yaml
schemaVersion: 1
job:
  job_id: job_batch_review_cycle
  state: enabled
  max_active_runs: 1
  default_input:
    base: agent-main
  steps:
    - target_type: activity
      target_id: review_batch_pr
      agent_cli: codex
      model: gpt-5.4
      condition: on_success
      timeout_seconds: 600

    - target_type: activity
      target_id: sync_batch_review_to_github
      condition: on_success
      timeout_seconds: 120

    - target_type: activity
      target_id: check_batch_review_decision
      condition: on_success
      timeout_seconds: 30

    - target_type: job
      target_id: job_batch_review_loop
      condition: on_failure
      timeout_seconds: 7200

    - target_type: activity
      target_id: merge_batch_pr
      condition: on_success
      timeout_seconds: 60
```
</details>


#### `job_review_tasks`

Standalone task review — evaluates proposed or in-progress tasks for quality, completeness, and acceptance criteria.

```bash
orbit job run job_review_tasks
```

<details>
<summary>Job YAML</summary>

```yaml
schemaVersion: 1
job:
  job_id: job_review_tasks
  state: enabled
  steps:
    - target_type: activity
      target_id: review_tasks
      agent_cli: codex
      model: gpt-5.4
      timeout_seconds: 1200
      env_extra: []
```
</details>

---

## Architecture

Orbit is structured as a layered set of Rust crates. Lower layers have no knowledge of higher layers.

```
orbit-types          (leaf — shared types, OrbitError, ID generation)
    ↑
orbit-policy         (RBAC policy evaluation)
orbit-exec           (process spawning, sandboxing, timeouts)
    ↑
orbit-tools          (builtin tool registry: fs, git, github, orbit, proc, time, net)
    ↑
orbit-store          (file-based YAML + SQLite persistence, layered store)
orbit-agent          (agent provider abstraction: Claude, Codex, mock)
    ↑
orbit-engine         (activity/job execution, template rendering, retry logic)
    ↑
orbit-core           (runtime bootstrap, config layering, command dispatch, asset seeding)
    ↑
orbit-cli            (CLI entry point, clap-based commands, JSON/table output)
```

### Model Strategy

Orbit uses a multi-model strategy to balance reasoning depth against throughput:

| Model | Role | Rationale |
| :--- | :--- | :--- |
| **Claude (Opus)** | Planning, dispatch | High-order reasoning; architectural |
| **Codex (gpt-5.4)** | Implementation, code generation, code review | Good execution quality; better code review quality |

You can configure your own preferred models in the `.orbit/jobs/jobs/<job_id>.yaml` file.

---

## Current Status

Orbit is in active development (WIP). Core execution primitives are stable for local use; production deployments are not yet recommended.

### Persistence Note

All Orbit state lives in `.orbit/` and is local by default. For persistence across machines without polluting git history, consider a Git overlay tool such as **monodev**. Orbit state overlays are tracked here:

- https://github.com/danieljhkim/orbit/tree/monodev/persist/persist/stores/orbit-states/overlay/.orbit

---

## Contributing

Contributions focused on core execution primitives, state serialization, or tool-calling interfaces are welcome. Open an issue or submit a pull request for review.
