# Platform-Agnostic Env Passthrough Design

**Goal:** Replace the static env allowlist with a principled, platform-aware mechanism.
**Scope:** `ExecutionEnvPolicy`, config schema, job/agent env setup. No changes to job scheduling.

## Approach Options (to evaluate)

### Option A: Platform-detected system var sets
- At startup, detect the platform and merge a platform-specific set of "system" vars
  into the base allowlist (e.g., macOS gets `TMPDIR`, `__CF_USER_TEXT_ENCODING`, `USER`;
  Linux gets `TMPDIR`, `USER`, `LANG`).
- Pros: Automatic, no user config needed.
- Cons: Still a static list per platform, just organized better.

### Option B: Per-job `env_extra` config
- Add an `env_extra` field to job definitions allowing additional vars per job.
- Pros: Flexible, no global allowlist bloat.
- Cons: Requires job authors to know what their agent CLI needs.

### Option C: Hybrid (recommended to evaluate)
- Platform-detected base set + per-job overrides.
- Best of both: automatic for common cases, configurable for edge cases.

## Steps

1. Research: Survey what env vars macOS, Linux, and common agent CLIs require.
2. Design: Write a short design doc (can be in-task) for the chosen approach.
3. Implement: Refactor `ExecutionEnvPolicy` to support platform detection and per-job overrides.
4. Test: Platform-conditional tests for macOS and Linux default sets.
5. Document: Update config.toml docs and CLAUDE.md if needed.

## Done When
- Hermetic mode works on macOS and Linux without manual config for standard agent CLIs.
- Per-job env overrides are supported.
- The static `DEFAULT_ENV_PASS` no longer contains platform-specific vars.