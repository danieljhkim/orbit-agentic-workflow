# Add Missing Orbit Tools

**Goal:** Register all 8 missing operations as built-in tools in orbit-tools so agents can use `orbit tool run <name>` uniformly.
**Scope:** New tool structs + registration only. No changes to CLI commands or store.
**Assumptions:** All target CLI commands support `--json` output (or none needed for archive). Verify before implementing.
**Risks:** orbit identity and orbit job-run --json flags may not exist yet — check before coding.

## Task 1: Verify CLI `--json` support for each target command

**Steps:**
1. Run `orbit task add --help`, `orbit task approve --help`, `orbit task reject --help`
2. Run `orbit identity list --help`, `orbit identity show --help`
3. Run `orbit job-run list --help`, `orbit job-run show --help`, `orbit job-run archive --help`
4. Note which commands support `--json` and which do not.
5. If `--json` is missing from any command, create a follow-up task to add it before proceeding.

**Done When:**
- All 8 commands confirmed to have `--json` output (or ticket filed for the gap).

## Task 2: Implement task tools (task.add, task.approve, task.reject)

**Files:**
- Create: `orbit-tools/src/builtin/orbit/task_add.rs`
- Create: `orbit-tools/src/builtin/orbit/task_approve.rs`
- Create: `orbit-tools/src/builtin/orbit/task_reject.rs`
- Modify: `orbit-tools/src/builtin/orbit/mod.rs`

**Pattern** (follow task_show.rs exactly):
- Struct implements `Tool` trait
- `schema()` returns `ToolSchema` with name, description, parameters
- `execute()` validates input with `required_string()`/`optional_string()`, builds args vec, calls `run_orbit_json_command()`
- For `task.add`: required fields are `title`, `description`, `plan`, `workspace`, `proposed_by`; optional: `context`, `assigned_to`, `created_by`, `priority`, `type`. Run `orbit task add ... --json` then return created task JSON.
- For `task.approve`/`task.reject`: required `id`, `by`, `note`. Run `orbit task approve/reject <id> --by <by> --note <note> --json`.
- Add `pub mod task_add; pub mod task_approve; pub mod task_reject;` to mod.rs
- Add 3 `registry.register(...)` calls in `register()`

**Steps:**
1. Write failing unit tests in mod.rs for `build_exec_request` of each new tool (input → expected args vec)
2. Run: `cargo test -p orbit-tools -- orbit 2>&1 | tail -20`
3. Implement the three tool structs
4. Re-run: `cargo test -p orbit-tools -- orbit 2>&1 | tail -20`

**Done When:** Tests pass, tools appear in `orbit tool list`.

## Task 3: Implement identity tools (identity.list, identity.show)

**Files:**
- Create: `orbit-tools/src/builtin/orbit/identity_list.rs`
- Create: `orbit-tools/src/builtin/orbit/identity_show.rs`
- Modify: `orbit-tools/src/builtin/orbit/mod.rs`

**Pattern:**
- `identity.show`: required `id`, delegates to `orbit identity show <id> --json`
- `identity.list`: optional `role` (engineer|CEO|leader), delegates to `orbit identity list [--role <role>] --json`

**Steps:**
1. Write failing unit tests
2. Run targeted test
3. Implement structs
4. Re-run test

**Done When:** Tests pass.

## Task 4: Implement job-run tools (job_run.list, job_run.show, job_run.archive)

**Files:**
- Create: `orbit-tools/src/builtin/orbit/job_run_list.rs`
- Create: `orbit-tools/src/builtin/orbit/job_run_show.rs`
- Create: `orbit-tools/src/builtin/orbit/job_run_archive.rs`
- Modify: `orbit-tools/src/builtin/orbit/mod.rs`

**Pattern:**
- `job_run.list`: optional `status` filter, delegates to `orbit job-run list [--status <status>] --json`
- `job_run.show`: required `id`, delegates to `orbit job-run show <id> --json`
- `job_run.archive`: required `id`, delegates to `orbit job-run archive <id>`. May not return JSON — return `{"archived": true, "id": "<id>"}` synthesized response if command exits 0.

**Steps:**
1. Write failing unit tests
2. Run targeted test
3. Implement structs
4. Re-run test

**Done When:** Tests pass.

## Task 5: Update registration test and run full suite

**Files:**
- Modify: `orbit-tools/src/builtin/orbit/mod.rs` (update `orbit_tools_are_registered` test)
- Modify: `orbit-cli/tests/tool_commands.rs` if integration test asserts tool count

**Steps:**
1. Add all 8 new tool names to `orbit_tools_are_registered()` assertions
2. Run: `cargo test --workspace 2>&1 | tail -30`
3. Fix any failures

**Done When:** `cargo test --workspace` passes.

## Final Verification
- `cargo test --workspace`
- `orbit tool list` shows all 8 new tools with `ENABLED=yes` and `BUILTIN=yes`
- Spot-check: `orbit tool run orbit.task.approve --input '{"id": "T1", "by": "Name", "note": "ok"}\' --dry-run`