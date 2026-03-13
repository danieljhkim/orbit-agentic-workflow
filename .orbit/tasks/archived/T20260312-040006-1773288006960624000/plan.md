# Repo-Local Orbit Root Plan

**Goal:** Remove implicit HOME-level .orbit support and make repository-local .orbit the default Orbit root.
**Scope:** Root resolution, initialization, config templates, bootstrap logic, tests, and docs tied to HOME-level behavior.
**Assumptions:** Existing repo-local config template is the intended long-term default; explicit root override behavior may remain if still useful.
**Risks:** Commands invoked outside git repos need a clear replacement behavior; multiple code paths currently distinguish between data_root and orbit_home.

## Task 1: Simplify root resolution and bootstrap behavior

**Files:**
- Modify: orbit-core/src/runtime/mod.rs
- Modify: orbit-core/src/paths.rs
- Modify: orbit-core/src/command/init.rs
- Modify: orbit-core/src/runtime/builder.rs

**Steps:**
1. Remove implicit selection of ~/.orbit when no explicit root override is provided.
2. Make runtime initialization resolve to repo-local .orbit when inside a git repo.
3. Replace HOME-only bootstrap behavior with repo-local initialization rules.
4. Define and implement the expected error path for commands run outside a repo without an explicit root.

**Done When:**
- Orbit no longer silently reads from or bootstraps ~/.orbit during normal startup.

## Task 2: Align config and identity defaults with repo-local roots

**Files:**
- Modify: orbit-core/src/config/runtime.rs
- Modify: orbit-core/src/config/bootstrap.rs
- Modify: orbit-core/assets/config/default-config.toml
- Modify: orbit-core/assets/config/default-config-repo.toml
- Modify: orbit-cli/src/command/config.rs

**Steps:**
1. Remove remaining config defaults that assume ~/.orbit as the implicit root.
2. Simplify template selection if separate home-vs-repo templates are no longer needed.
3. Ensure identity, skill, task, job, and audit paths resolve from the repo-local root by default.
4. Update config show output if the root/home distinction is no longer meaningful.

**Done When:**
- Config defaults and reporting reflect a repo-local-only Orbit model.

## Task 3: Update tests and documentation

**Files:**
- Modify: orbit-cli/tests/init_commands.rs
- Modify: orbit-cli/tests/config_commands.rs
- Modify: orbit-core/src/runtime/mod.rs tests
- Modify: README.md
- Modify: any additional tests or docs that assert HOME-level .orbit behavior

**Steps:**
1. Replace tests that expect ~/.orbit bootstrap with repo-local expectations.
2. Add or update coverage for failure behavior outside git repos when no explicit root is set.
3. Update README and command-facing docs so persistence/config examples use repository-local .orbit paths only.

**Done When:**
- Tests and docs no longer describe or rely on HOME-level .orbit support.

## Final Verification
- cargo test -p orbit-cli init_commands
- cargo test -p orbit-cli config_commands
- cargo test -p orbit-core runtime
- cargo test -p orbit --test init_commands
- cargo test -p orbit --test config_commands