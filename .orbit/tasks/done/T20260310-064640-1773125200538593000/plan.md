# Default Job Visibility Investigation Follow-Up

**Goal:** Make seeded default jobs discoverable after `orbit init`.
**Scope:** Job listing UX and any supporting init or job-state behavior needed to align the default experience.
**Assumptions:** Manual jobs should remain non-auto-scheduled, but they should still be visible enough that users can confirm they were created.
**Risks:** Changing list semantics too broadly could surface intentionally disabled jobs that users expect to stay hidden, so the fix should distinguish seeded defaults from generic disabled jobs if necessary.

## Task 1: Choose the intended UX

**Files:**
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-core/src/command/job.rs` if needed
- Modify: `README.md` or other CLI docs if needed

**Steps:**
1. Decide whether plain `orbit job list` should include disabled manual jobs, or whether init/output/docs should instead direct users to an alternate listing flow.
2. Keep the behavior consistent with the semantics of manual jobs and disabled jobs elsewhere in the product.
3. Update user-facing help or init output so the resulting behavior is understandable.

**Done When:**
- A user can reliably confirm seeded default jobs after init using the documented default workflow.

## Task 2: Implement and test the chosen fix

**Files:**
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-core/src/command/job.rs`
- Modify: `orbit-store/src/file/job_store.rs` if filtering semantics change
- Modify: `orbit-store/src/sqlite/job_store.rs` if filtering semantics change
- Test: `orbit-cli/tests/init_commands.rs`
- Test: `orbit-cli/tests/job_commands.rs`

**Steps:**
1. Add a failing regression test that seeds default jobs via `orbit init` and asserts the intended visibility behavior in `orbit job list`.
2. Implement the minimal change to align list behavior, init output, or both.
3. Re-run targeted init and job CLI tests.

**Done When:**
- Seeded default jobs are discoverable in the expected list flow and the behavior is covered by tests.

## Final Verification
- `cargo test -p orbit --test init_commands -- --nocapture`
- `cargo test -p orbit --test job_commands -- --nocapture`