# Repo-Local Orbit Default — Implementation Plan

**Goal:** Remove the $HOME/.orbit implicit fallback from all root resolution and init paths, making repo-local .orbit the unconditional default.
**Scope:** paths.rs, runtime resolution, init command, config templates, tests, CLAUDE.md.
**Assumptions:** git repo detection (find_git_repo_root) is correct and stays as-is.
**Risks:** Existing users with only $HOME/.orbit state will see Orbit behave as uninitialized; this is intentional and documented as out of scope for migration.

---

## Task 1: Update orbit_home_root and paths primitives

**Files:**
- Modify: orbit-core/src/paths.rs

**Steps:**
1. Write a failing test proving orbit_home_root no longer returns $HOME/.orbit.
   Run: cargo test -p orbit-core orbit_home_root -- --test-threads=1
2. Remove or repurpose orbit_home_root() — it should return CWD/.orbit or be deleted if unused after later steps.
3. Remove home_dir() and home_dir_required() only after confirming no remaining callers (check after all steps, not now).
4. Re-run: cargo test -p orbit-core -- --test-threads=1

**Done When:**
- No function in paths.rs returns $HOME/.orbit as a default path.

---

## Task 2: Rewrite resolve_initialize_data_root in runtime/mod.rs

**Files:**
- Modify: orbit-core/src/runtime/mod.rs

**Steps:**
1. Write failing tests for the new resolution order:
   - git repo without config → repo_root/.orbit
   - outside git repo → CWD/.orbit
   - ORBIT_ROOT env var still wins over both
   - --root CLI flag still wins over all
   Run: cargo test -p orbit-core resolve_initialize -- --test-threads=1
2. Rewrite resolve_initialize_data_root():
   - Remove home config fallback (steps 4 and 5 in current algorithm)
   - Step 3: if inside git repo, use repo_root/.orbit (regardless of config.toml presence)
   - Step 4 (new): fall back to cwd/.orbit
3. Remove should_bootstrap_orbit_home() or make it always return false (and remove callers).
4. Remove orbit_home_root() alias from this file; remove default_data_root() if it pointed to HOME.
5. Re-run: cargo test -p orbit-core -- --test-threads=1

**Done When:**
- All new resolution-order tests pass.
- No test references $HOME/.orbit as a default path.

---

## Task 3: Update init command (core and CLI)

**Files:**
- Modify: orbit-core/src/command/init.rs
- Modify: orbit-cli/src/command/init.rs

**Steps:**
1. Write a failing test: orbit init outside a git repo initializes CWD/.orbit, not $HOME/.orbit.
   Run: cargo test -p orbit-core init -- --test-threads=1
2. In orbit-core/src/command/init.rs:
   - Update resolve_init_root() to remove the home_orbit_root() branch entirely.
   - Outside git repo: return cwd/.orbit.
   - Inside git repo: return repo_root/.orbit (unchanged).
3. In orbit-cli/src/command/init.rs:
   - Remove home_orbit_root() helper.
   - Update execute_without_runtime to pass CWD (not HOME) as the default fallback.
4. Remove resolve_init_target_from_root() branching on orbit_root == home_root; always use the repo config template (root = '.').
5. Re-run: cargo test -p orbit-core init -- --test-threads=1
   Run: cargo test -p orbit-cli init -- --test-threads=1

**Done When:**
- orbit init in a git repo creates \<repo\>/.orbit.
- orbit init outside a git repo creates \<cwd\>/.orbit.
- No reference to $HOME/.orbit in init code paths.

---

## Task 4: Consolidate config templates

**Files:**
- Modify/Delete: orbit-core/assets/config/default-config.toml
- Modify: orbit-core/assets/config/default-config-repo.toml (rename to default-config.toml if former deleted)

**Steps:**
1. Determine if default-config.toml (root = '~/.orbit') is still referenced after Task 3 changes.
   Run: grep -r 'default-config.toml' orbit-core/
2. If unused: delete it.
3. If still referenced: update it to use root = '.' (same as default-config-repo.toml).
4. Consolidate to a single config template if both now have identical content.

**Done When:**
- No config template references $HOME/.orbit.
- Single default config template with root = '.'.

---

## Task 5: Update tests across workspace

**Files:**
- Modify: orbit-core/src/runtime/mod.rs (existing unit tests)
- Modify: orbit-core/tests/ (integration tests)
- Modify: orbit-store/tests/ (if referencing ~/.orbit paths)
- Modify: orbit-cli/tests/ (if referencing ~/.orbit paths)

**Steps:**
1. Grep for ~/.orbit in all test files: grep -r '~/.orbit\|\.orbit' orbit-*/tests/ orbit-core/src/
2. For each test assuming HOME fallback: rewrite to use CWD or tmpdir/.orbit.
3. Delete tests for should_bootstrap_orbit_home and home_config_root_used_when_* behaviors that no longer exist.
4. Run: make test

**Done When:**
- make test passes with zero failures.
- No test fixture or assertion references $HOME/.orbit as a default.

---

## Task 6: Update CLAUDE.md

**Files:**
- Modify: CLAUDE.md

**Steps:**
1. Change 'All data stored under ~/.orbit/ by default' to 'All data stored under \<repo\>/.orbit/ (or \<cwd\>/.orbit/ outside a git repo) by default'.
2. Remove any other HOME-level .orbit references.

**Done When:**
- CLAUDE.md accurately describes repo-local persistence model.

---

## Final Verification

```bash
make fmt
make clippy
make test
grep -r 'HOME.*orbit\|orbit_home\|~/.orbit' orbit-core/src/ orbit-cli/src/ --include='*.rs' | grep -v test
```

No non-test Rust source should reference HOME-derived .orbit paths after these changes.