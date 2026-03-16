Simplified task attribution by deriving actors from runtime context and moving decision provenance into task history.

Summary of changes:
- removed top-level proposal/review decision fields from the task model and CLI output
- added structured task history entries and recorded proposal/review approvals and rejections there
- auto-derived created_by, assigned_to, and proposed_by from runtime provenance instead of CLI flags
- propagated agent attribution through activity executions and `orbit tool run ...` invocations
- removed redundant identity-related CLI and built-in Orbit tool arguments
- removed the temporary legacy-decision compatibility path from the file task store
- made task metadata deserialization strict so obsolete decision fields are rejected instead of being silently accepted

Files touched:
- orbit-types/src/task.rs
- orbit-types/src/lib.rs
- orbit-store/src/backend/contracts.rs
- orbit-store/src/backend/file_backends.rs
- orbit-store/src/file/task_store.rs
- orbit-core/src/command/task.rs
- orbit-core/src/command/job.rs
- orbit-core/src/executor/cli_command.rs
- orbit-core/src/lib.rs
- orbit-core/tests/agent_run_behavior.rs
- orbit-cli/src/command/task.rs
- orbit-cli/tests/task_commands.rs
- orbit-cli/tests/agent_commands.rs
- orbit-tools/src/builtin/orbit/mod.rs
- orbit-tools/src/builtin/orbit/task_add.rs
- orbit-tools/src/builtin/orbit/task_approve.rs
- orbit-tools/src/builtin/orbit/task_reject.rs
- orbit-core/assets/skills/orbit/SKILL.md
- orbit-core/assets/skills/orbit-create-task/SKILL.md
- orbit-core/assets/skills/orbit-approve-task/SKILL.md
- orbit-core/assets/skills/orbit-maintain-system/SKILL.md
- .orbit/skills/orbit/SKILL.md
- .orbit/skills/orbit-create-task/SKILL.md
- .orbit/skills/orbit-approve-task/SKILL.md
- .orbit/skills/orbit-maintain-system/SKILL.md

Validation:
- cargo fmt --all
- cargo test -p orbit-store get_task_errors_when_legacy_decision_fields_are_present -- --nocapture
- cargo test -p orbit-store
- cargo test --workspace