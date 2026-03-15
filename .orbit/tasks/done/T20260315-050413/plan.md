# Add user.name Config Property

**Goal:** Let users set their name in orbit.toml; use it everywhere 'human' is currently hardcoded.
**Scope:** Config parsing, RuntimeConfig, OrbitRuntime public accessor, command layer (task.rs), store layer. No schema changes to task files — 'human' is already a valid value, the name just replaces it.
**Assumptions:** Config file is `<data_root>/config.toml` (orbit.toml). Default remains 'human' if unset.
**Risks:** Low — additive config field with fallback default.

## Task 1: Add raw config field and parse it

**Files:**
- Modify: `orbit-core/src/config/raw.rs` (or wherever RawRuntimeConfig is defined)

Add optional `[user]` section:
```toml
[user]
name = "daniel"
```

```rust
#[derive(Deserialize, Default)]
pub(crate) struct RawUserSection {
    pub(crate) name: Option<String>,
}

// in RawRuntimeConfig:
pub(crate) user: Option<RawUserSection>,
```

## Task 2: Thread through RuntimeConfig and OrbitRuntime

**Files:**
- Modify: `orbit-core/src/config/runtime.rs`

Add `user_name: String` to `RuntimeConfig`, defaulting to `"human"`.

Expose it on `OrbitRuntime`:
```rust
pub fn user_name(&self) -> &str {
    &self.config.user_name
}
```

## Task 3: Replace hardcoded 'human' at call sites

**Files:**
- Modify: `orbit-core/src/command/task.rs` (lines ~72, ~156) — pass `runtime.user_name()`
- Modify: `orbit-store/src/file/task_store.rs` (lines ~228, ~430, ~437) — these need the name threaded in via a parameter or context struct
- Modify: `orbit-cli/src/command/task.rs` (lines ~375, ~407) — change `default_value = "human"` to read from config, or remove default and fall back in core

Check whether store-layer hardcodes can be removed by having the caller always supply the actor name, rather than the store guessing.

## Task 4: Update tests

**Files:**
- Modify: `orbit-core/src/lib.rs` (~line 460) — assert against config-supplied name
- Modify: `orbit-cli/tests/task_commands.rs` (~line 448) — update 'human' assertion

## Final Verification
```bash
# Ensure default still works (no config)
orbit task add --title 'test' --description 'x' --plan 'x' --proposed-by 'x'
orbit task show <id> | grep 'by: human'

# With config set
echo '[user]\nname = "daniel"' >> .orbit/config.toml
orbit task add --title 'test2' --description 'x' --plan 'x' --proposed-by 'x'
orbit task show <id> | grep 'by: daniel'

cargo test -p orbit-core -p orbit-cli -p orbit-store
```