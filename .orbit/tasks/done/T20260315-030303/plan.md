# Add --json to All Orbit CLI Mutating Commands

**Goal:** Every orbit CLI subcommand emits structured JSON when `--json` is passed.
**Scope:** orbit-cli only; no changes to orbit-core or orbit-types.
**Assumptions:** The `crate::output::json::print_pretty` helper and existing `*_to_json` functions cover all needed shapes.
**Risks:** Some delete/archive commands currently print nothing or just a confirmation string; need to decide on a canonical JSON shape for these (e.g. `{"id": "...", "status": "deleted"}`).

## Task 1: task add / task update

**Files:**
- Modify: `orbit-cli/src/command/task.rs` — `TaskAddArgs`, `TaskUpdateArgs`

**Steps:**
1. Add `#[arg(long)] pub json: bool` to `TaskAddArgs` and `TaskUpdateArgs`.
2. In `TaskAddArgs::execute`: after creating the task, branch on `self.json` to call `print_pretty(&task_to_json(&task))` vs existing text output.
3. In `TaskUpdateArgs::execute`: same pattern.
4. Run: `cargo test -p orbit-cli`.

**Done When:**
- `orbit task add ... --json` prints a JSON object.
- `orbit task update ... --json` prints a JSON object.

## Task 2: task approve / task reject

**Files:**
- Modify: `orbit-cli/src/command/task.rs` — `TaskApproveArgs`, `TaskRejectArgs`

**Steps:**
1. Add `pub json: bool` to both structs.
2. Branch output in execute impls.
3. Run: `cargo test -p orbit-cli`.

**Done When:**
- Both commands emit JSON with the updated task when `--json` is passed.

## Task 3: task archive / task unarchive / task delete

**Files:**
- Modify: `orbit-cli/src/command/task.rs` — `TaskArchiveArgs`, `TaskUnarchiveArgs`, `TaskDeleteArgs`

**Steps:**
1. Add `pub json: bool` to each struct.
2. For archive/unarchive: emit task JSON. For delete: emit `{"id": "...", "deleted": true}`.
3. Run: `cargo test -p orbit-cli`.

**Done When:**
- All three commands support `--json`.

## Task 4: activity delete / job delete

**Files:**
- Modify: `orbit-cli/src/command/activity.rs` — `ActivityDeleteArgs`
- Modify: `orbit-cli/src/command/job.rs` — `JobDeleteArgs`

**Steps:**
1. Add `pub json: bool` to each struct.
2. Emit `{"id": "...", "deleted": true}` on success when `--json` is passed.
3. Run: `cargo test -p orbit-cli`.

**Done When:**
- Both commands support `--json`.

## Final Verification
```
cargo test -p orbit-cli
cargo clippy -p orbit-cli -- -D warnings
orbit task add --title "json-test" --description "d" --plan "p" --proposed-by system --json
orbit task update <id> --title "json-test-2" --json
```