# Provisioning Plan

**Goal:** Register five default named inactive jobs and the triage-and-dispatch-task activity.
**Prerequisite:** T20260310-061202 must be complete — `orbit job run --help` must show `--task-id` and `orbit job add --job-id` must work before running any command below.
**Scope:** CLI commands only. No code changes.
**Risks:** If a job ID already exists from a previous partial run, the `--job-id` insert will error. Check with `orbit job list` first.

## Task 1: Provision default named jobs

```bash
orbit job add --job-id job-resolve-backlogged-task \
  --target-id resolve-backlogged-task --schedule manual --agent-cli claude --timeout 15m

orbit job add --job-id job-perform-maintenance \
  --target-id perform-maintenance --schedule manual --agent-cli claude --timeout 15m

orbit job add --job-id job-oversee-orbit-operations \
  --target-id oversee-orbit-operations --schedule manual --agent-cli claude --timeout 15m

orbit job add --job-id job-approve-task-leader \
  --target-id approve-task-leader --schedule manual --agent-cli claude --timeout 15m

orbit job add --job-id job-triage-and-dispatch-task \
  --target-id triage-and-dispatch-task --schedule manual --agent-cli claude --timeout 5m
```

Verify each: `orbit job show <job-id>` — confirm state is `disabled`, schedule is `manual`.

**Done When:**
- All five jobs visible in `orbit job list`
- Each shows state `disabled` and schedule `manual`

## Task 2: Register triage-and-dispatch-task activity

```bash
orbit activity add \
  --id triage-and-dispatch-task \
  --type task_dispatch \
  --description "CEO reviews backlog and dispatches the highest-priority task for execution" \
  --instruction "You are Steve (CEO). Your job is to pick the single best task from the current backlog and dispatch it for execution.

Step 1: Run 'orbit task list --status backlog --json' to get all backlog tasks. If the backlog is empty, output 'Backlog is empty. Nothing to dispatch.' and exit.

Step 2: For promising candidates, call 'orbit task show <id>' to read description, plan, and priority.

Step 3: Select the single task most valuable to execute next. Prioritize: critical/high priority first; tasks that unblock others or reduce systemic risk; tasks with a clear executable plan; smaller contained changes over large sweeping ones.

Step 4: Run 'orbit job run job-resolve-backlogged-task --task-id <selected_task_id>' to dispatch.

Step 5: Record rationale: 'orbit task update <selected_task_id> --comment "Dispatched by Steve (CEO): <one sentence rationale>"'

Output: Dispatched <task_id>: <task_title>" \
  --skill-refs "orbit-skills,orbit-manage-tasks" \
  --identity steve \
  --assigned-to "Steve (CEO)" \
  --created-by "Steve (CEO)"
```

**Done When:**
- `orbit activity show triage-and-dispatch-task` shows identity `steve`
- Instruction references `job-resolve-backlogged-task` by name

## Task 3: Smoke test

```bash
orbit task list --status backlog          # confirm at least one task exists
orbit job run job-triage-and-dispatch-task
orbit job run --help | grep task-id       # flag must be present
orbit job history job-resolve-backlogged-task   # confirm a run was triggered with task_id set
```

**Done When:**
- `triage-and-dispatch-task` run completes without error
- A corresponding `resolve-backlogged-task` run appears in history
- The dispatched task has a CEO comment

## Final Verification
```bash
orbit job list                               # five default named jobs present
orbit activity show triage-and-dispatch-task
orbit job run job-triage-and-dispatch-task
```