Propagated task workspace through job_task_pipeline CLI steps so create_branch, run_tests, and checkout_branch can render working_dir reliably after dispatch_task.

Implemented runtime fallback in orbit-core/src/command/job.rs so template rendering uses input.workspace_path when an activity-local workspace_path is unset, which fixes the failing job-run shape we investigated.

Updated dispatch_task to return workspace_path explicitly and documented optional workspace_path input on the downstream CLI activity assets in both bundled and live Orbit activity definitions.

Added regressions covering agent-to-CLI workspace handoff and asset contract expectations.

Validation:
- cargo test -p orbit-core --test job_runtime_behavior agent_step_workspace_path_flows_into_cli_working_directory -- --nocapture
- cargo test -p orbit-core --test asset_formatting -- --nocapture
- cargo test --workspace