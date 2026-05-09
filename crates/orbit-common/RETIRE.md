# Retirement plan for `orbit-common`

`orbit-common` was split into two leaf crates as of task **T20260509-8**:

- **`orbit-util`** — generic helpers (filesystem, redaction, logging, blob
  storage, git, selectors, path normalization). Has no internal `orbit-*`
  dependencies.
- **`orbit-types`** — domain model (`OrbitError`, `OrbitId`, `Task`,
  `AuditEvent`, activity/job schemas, `Skill`, `FrictionEntry`, `TaskPlan`,
  `ExecutorDef`, `PolicyDef`, …) plus the Groundhog chronicle module.
  Depends only on `orbit-util`.

`orbit-common` now exists only as a thin re-export shim so existing
`orbit_common::types::*`, `orbit_common::utility::*`, and
`orbit_common::groundhog::*` import paths continue to resolve while consumers
migrate to importing from `orbit-types` / `orbit-util` directly.

This file is the **retirement plan** required by acceptance criterion #6 of
T20260509-8.

## Phases

### Phase 1 — *complete* (this PR)

Created `crates/orbit-types/` and `crates/orbit-util/`. Reduced
`crates/orbit-common/` to a re-export facade. Updated `Cargo.toml` workspace
members, `CLAUDE.md`, and `scripts/check-dependency-direction.sh`.

### Phase 2 — consumer migration (one PR per crate)

Migrate each consumer crate off `orbit-common`. The consumers are:

- `orbit-policy`, `orbit-exec`, `orbit-knowledge`, `orbit-store`,
  `orbit-registry`
- `orbit-tools`
- `orbit-mcp`
- `orbit-agent`
- `orbit-engine`
- `orbit-core`
- `orbit-cli`

For each crate, in a single PR:

1. In the manifest, replace `orbit-common = { path = ... }` with whichever of
   `orbit-types` / `orbit-util` is actually imported (usually both).
2. Run sed across the crate's `src/`, `tests/`, and `examples/`:
   - `orbit_common::types::` → `orbit_types::`
   - `orbit_common::utility::` → `orbit_util::`
   - `orbit_common::groundhog::` → `orbit_types::groundhog::`
   - `orbit_common::tracing` → `tracing` (or `orbit_types::tracing`)
   - bare `orbit_common::` (e.g. `orbit_common::OrbitError`) → `orbit_types::`
3. In `scripts/check-dependency-direction.sh`, remove `orbit-common` from
   the crate's `allowed_internal_deps` line.
4. Verify: `cargo build`, `cargo test -p <crate>`, and
   `bash scripts/check-dependency-direction.sh`.

The consumer migrations are mechanical and order-independent — the shim
keeps the workspace green between PRs.

### Phase 3 — delete `orbit-common`

Merge gate: `rg "orbit_common|orbit-common" -t rust crates/ scripts/ docs/`
returns no hits outside this `RETIRE.md` and the deletion PR's own diff.

In a single PR:

1. `rm -rf crates/orbit-common/`.
2. Drop `crates/orbit-common` from the workspace `members` list in the root
   `Cargo.toml`.
3. Drop the `orbit-common` case from `allowed_internal_deps()` and the
   `orbit-common` entry from `workspace_crates` in
   `scripts/check-dependency-direction.sh`.
4. Strike the `orbit-common` bullet from `CLAUDE.md`'s `## Crate
   Architecture` section (and remove the *transitional shim* note).
5. Verify: `cargo build` and `bash scripts/check-dependency-direction.sh`.

## Conventions during the transition window

- New code MUST import from `orbit-types` / `orbit-util` directly. Do not add
  new `orbit-common` import sites — that just grows the Phase 2 sed surface.
- `orbit-util` MUST NOT depend on `orbit-types`. The dep-direction guard
  enforces this; keep it that way. The shared `tracing` re-export
  intentionally lives in `orbit-types` (not `orbit-util`) to make the
  prohibition harder to violate by accident.
- The `OrbitError`-aware redaction helper
  (`redact_sensitive_env_error`) lives in `orbit-types::error`. The
  `orbit_common::utility::redaction` legacy path is preserved by re-exporting
  it through the shim's `utility::redaction` submodule.
