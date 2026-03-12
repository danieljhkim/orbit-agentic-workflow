# Restore Approval Scheduler Health

**Goal:** Return `approve-task-leader` automation to a clear, healthy operating state.
**Scope:** Audit both paused approval jobs, decide which job should remain active, complete recovery work, and close the tracking gap that allowed the earlier issue to be archived prematurely.
**Assumptions:** The later successful approval runs indicate the workflow itself can succeed, but the scheduler pause state and job duplication were never cleaned up.
**Risks:** Resuming the wrong job could duplicate approvals; archiving a job without confirming ownership could remove intended automation.

## Task 1: Confirm intended approval-job topology

**Files:**
- Review: `.orbit/jobs/jobs/job-1773034473007480000.yaml`
- Review: `.orbit/jobs/jobs/job-1773033071102526000.yaml`
- Review: archived issue `T20260310-033413-1773113653720463000`
- Review: `.orbit/agents/reports/2026-03-10/operation_oversee-orbit-operations.md`

**Steps:**
1. Determine why two `approve-task-leader` jobs exist and which one is supposed to own ongoing automation.
2. Confirm whether the provider-usage-limit incident is still blocking safe resume.
3. Document the correct operator action for the current paused primary job.

**Done When:**
- The intended active approval job and the reason for each paused job state are documented.

## Task 2: Recover and harden operations tracking

**Files:**
- Review and modify the operational artifacts or runtime modules identified in Task 1
- Test: targeted verification for job lifecycle/reporting if code changes are needed

**Steps:**
1. Resume or re-enable the correct approval job once safe, or explicitly archive/decommission jobs that should stay inactive.
2. Ensure there is an active issue/task until the recovery is complete so operational audits do not archive unresolved incidents.
3. Add any follow-up guardrails needed so future approval-job pauses remain visible and tracked.

**Done When:**
- Approval automation has one clear owner job in the correct state and unresolved pauses remain actively tracked.

## Final Verification
- `orbit job show job-1773034473007480000 --json`
- `orbit job show job-1773033071102526000 --json`
- `orbit task show <new_task_id> --json`