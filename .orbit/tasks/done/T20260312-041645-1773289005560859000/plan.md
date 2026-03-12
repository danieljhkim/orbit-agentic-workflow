# 20-Minute Job Timeout Plan

**Goal:** Increase the default Orbit job timeout from 15 minutes to 20 minutes.
**Scope:** CLI defaults, seeded default jobs, checked-in repo defaults, and tests/docs that assert the current 15-minute value.
**Assumptions:** 20 minutes should apply to newly created jobs and the default named jobs seeded by Orbit.
**Risks:** Existing tests and checked-in seeded job definitions currently expect or persist 900-second timeouts.

## Task 1: Update default timeout sources

**Files:**
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-core/src/command/job.rs`
- Modify: any additional code paths that encode the 900-second default for jobs

**Steps:**
1. Change the `orbit job add` default timeout from `15m` to `20m`.
2. Change seeded default named jobs from `900` seconds to `1200` seconds.
3. Verify the runtime and persistence layers continue to pass the configured timeout through unchanged.

**Done When:**
- New default-created jobs resolve to a 1200-second timeout.

## Task 2: Align checked-in defaults, tests, and docs

**Files:**
- Modify: `.orbit/jobs/jobs/job-resolve-backlogged-task.yaml`
- Modify: `.orbit/jobs/jobs/job-perform-maintenance.yaml`
- Modify: `.orbit/jobs/jobs/job-oversee-orbit-operations.yaml`
- Modify: `.orbit/jobs/jobs/job-approve-task-leader.yaml`
- Modify: `.orbit/jobs/jobs/job-triage-and-dispatch-task.yaml`
- Modify: `orbit-cli/tests/job_commands.rs`
- Modify: `README.md`
- Modify: any additional docs/tests that describe the current 15-minute default

**Steps:**
1. Update checked-in seeded job YAMLs from `timeout_seconds: 900` to `timeout_seconds: 1200` where appropriate.
2. Rename and update tests that currently assert the 15-minute default.
3. Update documentation examples from `--timeout 15m` to `--timeout 20m` where the example is describing the default recommendation.

**Done When:**
- Tracked defaults, docs, and tests all reflect a 20-minute job timeout.

## Final Verification
- `cargo test -p orbit --test job_commands`
- Run any targeted tests that cover default named job seeding
- Sanity-check `orbit job add` without `--timeout` and confirm `timeout_seconds == 1200`