# Remove Abstract Tool Refs from Activity Definitions

**Goal:** Strip fs.* and orbit.task.* abstract tool names from tools lists and rewrite instruction text to use CLI commands directly.
**Scope:** activity YAML files under orbit-core/assets/activities/. No Rust changes needed.
**Assumptions:** Agents use proc.spawn or native shell access to run CLI commands. git.*, github.*, and proc.* tools remain valid and are not changed.
**Risks:** Instruction text in implement_change.yaml is detailed — must preserve meaning while changing tool references.

## Step 1: Update implement_change.yaml

**File:** `orbit-core/assets/activities/implement_change.yaml`

**tools: list — remove:**
- orbit.task.show, orbit.task.update, orbit.task.list
- fs.read, fs.list, fs.write, fs.delete

**Instruction text — rewrite affected lines:**
- "Call orbit.task.show with id: ..." → "Run: orbit task show <task_id> --json"
- "Call orbit.task.update with ... status: in-progress" → "Run: orbit task update <task_id> --status in-progress"
- "Use fs.list and fs.read to read the files listed in contextFiles" → "Read the files listed in contextFiles using standard shell commands"
- "Write code with fs.write" → "Write code using your native file editing capabilities"
- "Call orbit.task.update with ... status: review ... execution_summary: ..." → "Run: orbit task update <task_id> --status review --execution-summary ..."}"
- "orbit.task.update to add a comment" → "orbit task update <task_id> --comment ..."

**Done When:** tools list contains only git.stage_paths, git.commit, proc.spawn, proc.which; no fs.* or orbit.task.* in tools or instruction text.

## Step 2: Update dispatch_task.yaml

**File:** `orbit-core/assets/activities/dispatch_task.yaml`

**tools: list — remove:** orbit.task.list, orbit.task.show, orbit.task.update

**Instruction text — rewrite:**
- "Call orbit.task.list with status: backlog" → "Run: orbit task list --status backlog --json"
- "call orbit.task.show with the task id" → "Run: orbit task show <id> --json"
- "Call orbit.task.update with id: ... and comment:" → "Run: orbit task update <id> --comment ..."

**Done When:** tools list is empty ([]); instruction text uses CLI command syntax.

## Step 3: Update open_pr.yaml and review_pr.yaml

**Files:** `orbit-core/assets/activities/open_pr.yaml`, `orbit-core/assets/activities/review_pr.yaml`

Remove orbit.task.show from tools list in both files. Update any instruction text references to use `orbit task show <id> --json` instead.

**Done When:** orbit.task.show absent from both files' tools lists.

## Final Verification
```
grep -r 'fs\.\|orbit\.task\.' orbit-core/assets/activities/
cargo build -p orbit-core
```
Both should produce no matches / no errors.