Added a runtime-level `user.name` config property loaded from `.orbit/config.toml` and used it as the canonical fallback actor for task attribution.

Summary of changes:
- added `[user] name` parsing and validation to runtime config with `human` as the default
- exposed the configured user name on `OrbitRuntime`
- threaded explicit task mutation actors through the core task command path into the file-backed task store so history entries no longer guess `human` internally
- defaulted `created_by` to the configured user name during task creation while preserving explicit `created_by` / `proposed_by` behavior for initial comments
- updated CLI approve/reject commands to fall back to the configured user name when `--by` is omitted
- added regression coverage for config parsing, runtime attribution, CLI task flows, and store history actor persistence

Validation:
- cargo test -p orbit-core
- cargo test -p orbit-store
- cargo test -p orbit-cli --test task_commands