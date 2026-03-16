Refactored the job runtime out of command/job.rs into a dedicated internal engine/executor layout inside orbit-core.

Summary of changes:
- added orbit-core/src/engine/{mod,activity_runner,job_runner}.rs to own job-run orchestration, activity dispatch, step-input propagation, and stale-run recovery
- added orbit-core/src/executor/agent.rs so agent invocation/protocol handling no longer lives in command/job.rs
- reduced orbit-core/src/command/job.rs to job CRUD/default-job seeding plus thin runtime delegation through OrbitRuntime methods implemented elsewhere
- removed the legacy created_file special-case and deleted execute_created_file_auto_commit entirely
- updated bundled and live maintenance activity/skill artifacts so Orbit no longer advertises report auto-commit behavior
- removed the obsolete created_file regression tests and kept the runtime coverage focused on the supported execution contract

Validation:
- cargo fmt --all
- cargo test -p orbit-core --test job_runtime_behavior -- --nocapture
- cargo test -p orbit-core --test asset_formatting -- --nocapture
- cargo test --workspace

Notes:
- the existing lock_store dead-code warning in orbit-core/src/context.rs is still present and unrelated to this refactor