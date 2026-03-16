Extracted the job/activity execution stack into a new `orbit-engine` crate and rewired `orbit-core` to use it as the runtime engine.

Summary of changes:
- added the new `orbit-engine` workspace crate with runner, executor, template, and schema modules
- introduced an `EngineHost` trait and implemented it on `OrbitRuntime` inside a new core runtime adapter
- moved job-run orchestration and direct activity execution out of `orbit-core/src/engine/*`
- removed the old in-core `engine` and `executor` modules after the new crate became authoritative
- preserved agent required-env preflight, automation worktree/commit/PR behavior, step-output propagation, and stale-run recovery across the extraction

Validation:
- cargo check --workspace
- cargo test -p orbit-engine
- cargo test -p orbit-core --test job_runtime_behavior -- --nocapture
- cargo test -p orbit-core --test asset_formatting -- --nocapture
- cargo test --workspace
- cargo fmt --all