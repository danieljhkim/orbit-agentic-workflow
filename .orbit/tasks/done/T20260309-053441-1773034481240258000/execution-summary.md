Implemented task comment append support across the shared task model, file-backed task store, runtime, and CLI.

Summary of changes:
- added shared `TaskComment` and exposed `comments` on `Task`
- extended file-backed task create/update contracts to persist initial and appended comments
- added `--comment` to `orbit task add`, `update`, `approve`, and `reject`
- appended comments deterministically using existing identities where available and `human` for update commands
- exposed comments in `task show` JSON and human-readable output
- added store, runtime, and CLI regression coverage for add/update/approve/reject comment flows and blank comment rejection

Files touched:
- orbit-types/src/task.rs
- orbit-types/src/lib.rs
- orbit-store/src/backend/contracts.rs
- orbit-store/src/backend/file_backends.rs
- orbit-store/src/file/task_store.rs
- orbit-core/src/command/task.rs
- orbit-core/src/lib.rs
- orbit-core/tests/agent_run_behavior.rs
- orbit-cli/src/command/task.rs
- orbit-cli/tests/task_commands.rs

Validation:
- cargo test -p orbit-store task_store
- cargo test -p orbit-core task_
- cargo test -p orbit task_