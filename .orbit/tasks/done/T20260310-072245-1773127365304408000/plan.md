# Fix Hermetic Env for macOS Agent Invocation

**Goal:** Agent CLIs spawned by the job runtime must not panic due to missing macOS system env vars.
**Scope:** Environment passthrough allowlist in config and defaults; no changes to job scheduling or agent protocol.
**Assumptions:** The hermetic model (`inherit = false`) is intentional and should be preserved. We expand the default allowlist rather than switching to full inheritance.
**Risks:** Over-permissive allowlist leaks sensitive env vars into agent processes. Keep additions minimal and documented.

## Task 1: Identify required macOS env vars

**Steps:**
1. Run `env` on macOS and identify vars set by the OS itself (not user/app vars).
2. Cross-reference with `SCDynamicStore` / CoreFoundation requirements.
3. Determine minimal set to add (likely: `TMPDIR`, `__CF_USER_TEXT_ENCODING`, `SHELL`, `USER`, `LOGNAME`).

**Done When:**
- A concrete list of vars is identified that prevents the Codex panic.

## Task 2: Expand default env passthrough allowlist

**Files:**
- Modify: `orbit-core/src/config/runtime.rs` (update `DEFAULT_ENV_PASS`)
- Modify: `orbit-core/assets/config/default-config.toml` (update `[execution.env] pass`)
- Modify: `.orbit/config.toml` (update local config to match)

**Steps:**
1. Add the identified macOS vars to `DEFAULT_ENV_PASS` in `runtime.rs`.
2. Update `default-config.toml` to match.
3. Update `.orbit/config.toml` to match.
4. `make build && make test`

**Done When:**
- Default allowlist includes the required macOS vars.
- All existing tests pass.

## Task 3: Add integration test for hermetic agent spawn on macOS

**Files:**
- Modify or create test in `orbit-exec/` or `orbit-core/` for environment clearing.

**Steps:**
1. Write a test that spawns a trivial process with `ClearAndSet` using only the default allowlist.
2. Verify the process can start and access basic system config without panic.
3. `cargo test -p orbit-exec <test_name>`

**Done When:**
- Test passes on macOS, confirming the allowlist is sufficient.

## Task 4: Verify fix end-to-end

**Steps:**
1. `orbit job run job-resolve-backlogged-task` completes without `AGENT_INVOCATION_FAILED`.
2. Run state is not `failed` with exit code 101.

## Final Verification
- `make ci`
- Manual: `orbit job run job-resolve-backlogged-task` succeeds or fails for a non-env reason.