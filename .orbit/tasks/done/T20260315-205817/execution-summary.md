Added eight new built-in Orbit tools under `orbit-tools/src/builtin/orbit/` and registered them in the built-in Orbit tool registry: `orbit.task.add`, `orbit.task.approve`, `orbit.task.reject`, `orbit.identity.list`, `orbit.identity.show`, `orbit.job_run.list`, `orbit.job_run.show`, and `orbit.job_run.archive`.

Matched the existing Orbit tool pattern with request-builder unit tests in `orbit-tools/src/builtin/orbit/mod.rs`, JSON-backed execution for the seven read/create/update-style commands, and a synthesized `{"archived": true, "id": ...}` success payload for `orbit.job_run.archive`, which does not expose `--json` on the CLI.

Rounded out the tool input surface a bit beyond the task draft where it improved consistency with the real CLI: `task.add` now supports optional `comment`, `context`, `assigned_to`, `created_by`, `priority`, and `type`; `task.approve` / `task.reject` support optional `comment`; `job_run.list` exposes the existing `job`, `status`, `since`, and `limit` filters.

Validation:
- `cargo fmt --package orbit-tools`
- `cargo test -p orbit-tools`
- `cargo test -p orbit-cli --test tool_commands`
- `cargo test --workspace`
- `cargo run -q -p orbit-cli --bin orbit -- tool list`
- `cargo run -q -p orbit-cli --bin orbit -- --root .orbit tool run orbit.identity.list --input '{"role":"engineer"}'`\n- `cargo run -q -p orbit-cli --bin orbit -- --root .orbit tool run orbit.identity.show --input '{"id":"linus"}'`\n- `cargo run -q -p orbit-cli --bin orbit -- --root .orbit tool run orbit.job_run.list --input '{}'`