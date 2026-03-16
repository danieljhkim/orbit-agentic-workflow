# orbit-engine Extraction Plan

**Goal:** Move job/activity execution orchestration and executors out of `orbit-core` and into a new `orbit-engine` crate with a clean service boundary.
**Scope:** workspace crate wiring, engine-owned shared types, executor extraction, activity/job runner extraction, core adapter glue, and regression coverage.
**Assumptions:** The phase-1 internal engine split in `orbit-core` is the staging seam, and `orbit-core` should remain the runtime/bootstrap composition root rather than disappearing.
**Risks:** Trait boundaries may be too coarse or too leaky on the first pass, and extracting the wrong shared types can create circular dependencies or unnecessary public API churn.

## Task 1: Create the new crate and define the service seam

**Files:**
- Create: `orbit-engine/Cargo.toml`
- Create: `orbit-engine/src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `orbit-core/Cargo.toml`
- Modify: `orbit-core/src/lib.rs`
- Modify: any shared type crates if a type must move lower in the dependency graph

**Steps:**
1. Add `orbit-engine` as a workspace crate with the minimal dependencies it truly needs.
2. Define the core engine facade and the service traits `orbit-core` will implement for engine execution.
3. Decide which shared structs belong in `orbit-engine` versus `orbit-types` versus adapters in `orbit-core`.
4. Keep the initial public surface intentionally small so the engine crate is not just a dump of internal helpers.

**Done When:**
- The workspace builds with a new `orbit-engine` crate.
- There is a clear trait/service seam for engine execution rather than direct `OrbitRuntime` coupling.

## Task 2: Move executors and engine-owned helper types

**Files:**
- Create/Modify: `orbit-engine/src/executor/*.rs`
- Create/Modify: `orbit-engine/src/context.rs` or equivalent shared-engine module
- Modify/Delete: `orbit-core/src/executor/agent.rs`
- Modify/Delete: `orbit-core/src/executor/automation.rs`
- Modify/Delete: `orbit-core/src/executor/cli_command.rs`
- Modify/Delete: `orbit-core/src/executor/api.rs`
- Modify: `orbit-core/src/executor/mod.rs`

**Steps:**
1. Extract engine-local shared types such as execution context, attempt outcomes, template context, and step merge helpers into the new crate.
2. Move the executor implementations into `orbit-engine`, rewriting them to depend on the new service traits instead of `OrbitRuntime`.
3. Keep behavior stable while reducing direct knowledge of stores, command modules, and runtime internals inside executor code.
4. Add or update targeted tests around the most coupled executor boundaries as the move lands.

**Done When:**
- The executors compile and run from `orbit-engine`.
- `orbit-core` no longer owns executor logic directly.

## Task 3: Move activity and job runners into orbit-engine

**Files:**
- Create/Modify: `orbit-engine/src/activity_runner.rs`
- Create/Modify: `orbit-engine/src/job_runner.rs`
- Modify/Delete: `orbit-core/src/engine/activity_runner.rs`
- Modify/Delete: `orbit-core/src/engine/job_runner.rs`
- Modify/Delete: `orbit-core/src/engine/mod.rs`
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Extract activity dispatch into `orbit-engine` so spec-type branching lives entirely there.
2. Extract job-run orchestration, retries, step input propagation, and finalization into `orbit-engine`.
3. Leave `orbit-core/src/command/job.rs` as a thin adapter that delegates into the engine crate.
4. Remove the temporary in-core engine modules once the crate-based engine is authoritative.

**Done When:**
- `orbit-core` delegates job execution to `orbit-engine` rather than owning runner logic itself.
- The old in-core engine modules are gone or reduced to minimal adapter shims.

## Task 4: Wire core adapters and validate the extraction end to end

**Files:**
- Modify: `orbit-core/src/lib.rs`
- Modify: `orbit-core/src/context.rs`
- Modify: any adapter or runtime modules needed to implement the engine service traits
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Modify: `orbit-core/tests/asset_formatting.rs`
- Modify: any crate-specific tests needed in `orbit-engine`

**Steps:**
1. Implement the engine service traits inside `orbit-core` using the existing stores, tools, and runtime assembly.
2. Keep audit/mutation boundaries and persistence semantics stable while the execution code moves crates.
3. Add crate-local tests in `orbit-engine` where they improve isolation, and keep the existing end-to-end runtime coverage in `orbit-core`.
4. Run focused and full-workspace validation to confirm the extraction did not change behavior.

**Done When:**
- `orbit-engine` is the runtime execution crate and `orbit-core` is the composition root.
- Existing job/activity regression coverage still passes after the move.

## Final Verification
- `cargo test -p orbit-engine`
- `cargo test -p orbit-core --test job_runtime_behavior -- --nocapture`
- `cargo test -p orbit-core --test asset_formatting -- --nocapture`
- `cargo test --workspace`