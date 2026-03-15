Removed the redundant `orbit_home` runtime/context surface and cleaned up config output so the active Orbit root is no longer described with HOME-specific naming.

Summary of changes:
- deleted `OrbitContext.orbit_home` and `OrbitRuntime::orbit_home()` because the production path already uses `data_root` as the canonical selected root
- simplified runtime builder wiring so only `data_root` is threaded through the runtime context
- updated `orbit config show` JSON/text output to expose `root` plus an explicit `selected_root` field and removed the old `home` field/`ORBIT_HOME` label
- strengthened config command tests to assert the new output contract and the absence of the old `home` key

Compatibility decision:
- did not keep a `home` JSON alias because it would preserve the misleading HOME-backed semantics this task was meant to remove
- retained the existing `root` field and added `selected_root` so consumers still have a stable path field while gaining clearer terminology

Validation:
- cargo test -p orbit-core
- cargo test -p orbit-cli --test config_commands
- rg -n "orbit_home|ORBIT_HOME|\bhome\b" orbit-core orbit-cli CLAUDE.md -g !target  # remaining matches are actual HOME env handling and test fixture names, not selected-root surfaces