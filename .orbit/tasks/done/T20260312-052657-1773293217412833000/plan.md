# Report Auto-Commit Implementation Plan

**Goal:** Have `job-perform-maintenance` and `job-oversee-orbit-operations` automatically commit their created report plus the current job-run artifact after a successful run.
**Scope:** Limit the behavior to these two report-producing jobs, their activity contracts, their skill instructions, and the runtime path that finalizes successful runs.
**Assumptions:** These jobs continue to write exactly one primary markdown report per successful run, and the persisted job-run artifact path remains derivable from the job ID and run ID.
**Risks:** If the commit still runs before completion persistence, Orbit will not be able to include the active run artifact. If path validation is weak, agents could point Orbit at unintended files. If seeded assets and repo-local skills diverge, behavior and guidance will drift.

## Task 1: Define the success output contract for report-producing jobs

**Files:**
- Modify: `orbit-core/assets/activities/perform-maintenance.yaml`
- Modify: `orbit-core/assets/activities/oversee-orbit-operations.yaml`
- Modify: `.orbit/skills/orbit-maintain-system/SKILL.md`
- Modify: `.orbit/skills/orbit-operations-management/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-maintain-system/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-operations-management/SKILL.md`

**Steps:**
1. Extend both activity output schemas to accept the existing `comment` plus a narrow field such as `created_file` for the generated markdown artifact.
2. Update both skill copies so the agent is instructed to return the created report path in that field after writing the file.
3. Keep the activity/skill wording explicit that Orbit, not the agent, performs the commit.
4. Run a focused activity-spec sanity check: `cargo test -p orbit-core bundled_default_activity_specs_parse_successfully -- --nocapture`

**Done When:**
- Both activities document and validate the created-file output contract.
- Repo-local and seeded skill copies stay aligned.

## Task 2: Commit only after the current run artifact exists

**Files:**
- Modify: `orbit-core/src/command/job.rs`
- Modify: `orbit-core/src/command/activity.rs` (only if helper data needs to move through the execution context)
- Test: `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Add or update a runtime test that proves a successful maintenance/operations-style result cannot commit until the current run has been completed and its YAML artifact exists.
2. Refactor the success path so Orbit persists the run completion first, then derives the active run artifact path, then invokes the existing typed git tools.
3. Build the commit file list from exactly two paths: the returned created file and the current run artifact.
4. Generate a deterministic commit message that includes the job ID and run ID for auditability.
5. Keep failure handling clear: invalid/missing created-file paths should fail the run without staging unintended files.

**Done When:**
- Successful runs of the targeted jobs auto-commit the created report and the matching run artifact.
- The commit path still stages only the explicit intended files.

## Task 3: Cover validation edges and seeded behavior

**Files:**
- Test: `orbit-core/tests/job_runtime_behavior.rs`
- Test: `orbit-cli/tests/init_commands.rs`
- Modify: any minimal supporting test fixtures needed for seeded activity refresh

**Steps:**
1. Add regression coverage for invalid created-file payloads (empty path, missing file, or path outside repo root).
2. Add coverage proving the commit contains the report and the current run artifact, and excludes unrelated staged files.
3. Verify seeded activity updates remain visible after init/refresh where applicable.
4. Run targeted verification: `cargo test -p orbit-core --test job_runtime_behavior -- --nocapture`
5. Run seeded-default verification: `cargo test -p orbit --test init_commands -- --nocapture`

**Done When:**
- The runtime behavior is protected by targeted tests.
- Seeded defaults and skill guidance stay consistent with the implemented contract.

## Final Verification
- `cargo test -p orbit-core --test job_runtime_behavior -- --nocapture`
- `cargo test -p orbit --test init_commands -- --nocapture`
- Manual spot-check in a temp repo if practical: run each job once, then inspect `git show --name-only HEAD` and confirm it contains only the created report plus the matching `.orbit/jobs/runs/<job_id>/<run_id>.yaml` file.