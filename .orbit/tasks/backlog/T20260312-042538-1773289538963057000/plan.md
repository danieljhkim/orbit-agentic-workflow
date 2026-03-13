# Fix Claude CLI Backend

**Goal:** Make Claude CLI agent jobs work correctly in Orbit's hermetic job execution.
**Scope:** `orbit-agent/src/providers/claude/` and `orbit-agent/src/providers/mod.rs`. No protocol or response-parsing changes.
**Assumptions:** `claude` CLI v2+ is on PATH. `ANTHROPIC_API_KEY` is available in the parent shell.
**Risks:** `--permission-mode bypassPermissions` gives Claude unrestricted tool access — acceptable in Orbit's sandboxed hermetic environment, but must be documented.

## Task 1: Add required flags to ClaudeCliTransport

**Files:**
- Modify: `orbit-agent/src/providers/claude/claude_cli.rs`
- Test: `orbit-agent/tests/protocol_behavior.rs`

**Steps:**
1. Add failing test asserting new args:
   ```rust
   assert_eq!(args, vec![
       "-p", "--permission-mode", "bypassPermissions",
       "--output-format", "text",
       "--no-session-persistence",
   ]);
   ```
2. Run: `cargo test -p orbit-agent provider_mapper_supports_claude`
3. Update `ClaudeCliTransport::args()` to include the new flags.
4. Re-run test → green.
5. Run: `cargo test -p orbit-agent`

**Done When:**
- `provider_mapper_supports_claude` passes with updated args
- All orbit-agent tests pass

## Task 2: Add `ANTHROPIC_API_KEY` to Claude required env vars

**Files:**
- Modify: `orbit-agent/src/providers/mod.rs`
- Test: `orbit-agent/tests/protocol_behavior.rs`

**Steps:**
1. Add failing test:
   ```rust
   #[test]
   fn claude_runtime_requires_anthropic_api_key() {
       let vars = AgentProvider::Claude.required_env_vars();
       assert!(vars.contains(&"ANTHROPIC_API_KEY"));
   }
   ```
2. Run test → red.
3. Update `required_env_vars` for `AgentProvider::Claude` to include `"ANTHROPIC_API_KEY"`.
4. Re-run → green.
5. Verify error message quality: if the key is missing from the allowlist, the job error should say "configure .orbit/config.toml [execution.env].pass".

**Done When:**
- Test passes.
- Running a Claude job without `ANTHROPIC_API_KEY` in the env allowlist fails with `AGENT_INVOCATION_FAILED` and the config guidance message — not a silent auth error from the claude binary.

## Task 3: Integration test — mock claude script

**Files:**
- Modify: `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Add a mock `claude` shell script (similar to the existing mock agent pattern) that:
   - Verifies `-p`, `--permission-mode bypassPermissions`, `--output-format text`, `--no-session-persistence` are all present in `$@`
   - Reads stdin, outputs a valid Orbit JSON envelope
2. Add test `claude_job_run_succeeds_with_mock_binary()` that:
   - Creates the mock script
   - Creates a job with `agent_cli: <path_to_mock_claude>`
   - Runs it
   - Asserts success state
3. Run: `cargo test -p orbit-core claude_job_run`

**Done When:**
- New integration test passes.
- Confirms full invocation path for Claude end-to-end.

## Final Verification
```bash
cargo test -p orbit-agent
cargo test -p orbit-core claude
cargo clippy -p orbit-agent -- -D warnings
cargo fmt -p orbit-agent --check
```