Normalized built-in activity and job naming to snake_case across bundled assets, tracked repo-local copies, loader registration, and init compatibility handling.

Summary of changes:
- renamed bundled activity assets and IDs to snake_case (`approve_task_leader`, `oversee_orbit_operations`, `perform_maintenance`, `resolve_backlogged_task`, `dispatch_task`) and updated bundled loader keys accordingly
- renamed bundled job assets and IDs to snake_case (`job_approve_task_leader`, `job_oversee_orbit_operations`, `job_perform_maintenance`, `job_resolve_backlogged_task`, `job_dispatch_task`, `job_execute_task`) and updated default job registration to match
- added an `orbit init` migration that rewrites legacy kebab-case built-in activity/job files and migrates legacy job run directories and step metadata into the new snake_case layout
- refreshed the tracked repo-local `.orbit` activity/job copies into the same snake_case naming scheme and aligned the regenerated `dispatch_task` / `job_dispatch_task` copies with the bundled source assets
- added CLI regression coverage proving a legacy kebab-case workspace upgrades cleanly, including the old `triage-and-dispatch-task` -> `dispatch_task` and `job-triage-and-dispatch-task` -> `job_dispatch_task` mapping plus preserved job history
- fixed the bundled `dispatch_task` instructions numbering drift (step 4 was mislabeled as step 5)
- fixed one stale CLI test that still referenced removed default identity `grace`, updating it to canonical `linus` so workspace validation passes cleanly

Files touched:
- orbit-core/src/command/activity.rs
- orbit-core/src/command/job.rs
- orbit-core/src/command/init.rs
- orbit-core/assets/activities/*
- orbit-core/assets/jobs/*
- .orbit/activities/active/*
- .orbit/jobs/jobs/*
- orbit-cli/tests/init_commands.rs
- orbit-cli/tests/job_commands.rs
- orbit-cli/tests/activity_commands.rs

Validation:
- cargo test -p orbit-cli --test init_commands
- cargo test -p orbit-cli --test activity_commands
- cargo test -p orbit-core
- cargo test --workspace
- rg -n for live kebab-case built-in references in source/assets/tests confirmed only the migration map and legacy test fixture strings remain

Notes:
- historical task bundles under `.orbit/tasks/*` were left unchanged even when they reference old names, since they are audit artifacts rather than live built-ins
- the currently installed `orbit` CLI regenerated repo-local built-in copies during task lifecycle commands; I aligned those regenerated snake_case copies before commit so the tracked `.orbit` state matches the source-tree migration