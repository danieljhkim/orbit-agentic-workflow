# Runtime Config Path Terminology Cleanup

**Goal:** Make live Orbit docs, comments, messages, and the active task record use one unambiguous name for the runtime config file.
**Scope:** Live user-facing config guidance and the current task record. Avoid rewriting unrelated historical task artifacts.
**Assumptions:** The on-disk contract remains unchanged: the runtime config file is `<data_root>/config.toml`, typically `.orbit/config.toml` in a repo-local workspace.
**Risks:** Low. The main risk is over-editing historical records instead of clarifying active guidance.

## Task 1: Find live wording drift
1. Search the repo for `orbit.toml` and related config-file wording.
2. Separate live guidance from historical task artifacts.

## Task 2: Normalize live wording
1. Update live user-facing docs/messages/comments to name the runtime config path consistently.
2. Update the current task record so it no longer implies a different filename than the implementation uses.

## Final Verification
- `rg -n "orbit\.toml|config\.toml" .orbit orbit-core orbit-cli`
- manual review that the remaining `orbit.toml` references are historical artifacts or intentional notes, not live guidance