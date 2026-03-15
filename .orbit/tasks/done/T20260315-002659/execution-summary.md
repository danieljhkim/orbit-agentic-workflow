Extended activities from a single agent-only path into typed execution modes backed by `spec_config`.

Summary of changes:
- replaced `Activity` instruction/skill_refs/tools fields with flattened `spec_config` across orbit-types, orbit-store, orbit-core, and orbit-cli
- validated `spec_type` against `agent_invoke`, `cli_command`, and `api`, and rewired the agent envelope to merge spec_config fields at runtime
- added a small template engine plus new `cli_command` and `api` executors in orbit-core
- extended orbit-exec with `current_dir` support so `cli_command.working_dir` can run through the shared process runner
- updated bundled default activity YAMLs to `agent_invoke` and refreshed CLI/runtime tests for the new schema
- added focused coverage that `cli_command` activities run without `agent_cli`, honor `working_dir`, and consume `ORBIT_OUTPUT_FILE`

Assumptions and follow-ups:
- `{{workspace_path}}` currently resolves to Orbit's data root/workspace path during execution; ambiguity was captured as follow-up task T20260315-004224

Validation:
- cargo check --workspace
- cargo test --workspace