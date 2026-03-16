Removed camelCase task bundle persistence and switched the file-backed task store to snake_case-only task YAML keys.

Manually rewrote every remaining live task bundle under `.orbit/tasks/backlog/`, `.orbit/tasks/in_progress/`, and `.orbit/tasks/proposed/` so the active repository state matches the new format. Also normalized the one surviving legacy comment entry from `text` to `message` so it conforms to the shared task schema.

Added store-level assertions proving new task bundles and rewrites emit snake_case keys and no longer serialize camelCase aliases.

Validation:
- cargo test -p orbit-store
- cargo test -p orbit-types
- cargo test --workspace
- cargo run -q -p orbit-cli --bin orbit -- --root .orbit task list --json
- cargo run -q -p orbit-cli --bin orbit -- --root .orbit task show T20260315-183553