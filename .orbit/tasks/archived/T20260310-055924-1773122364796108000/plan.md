# Resolve malformed backlog-job success record

**Goal:** Restore confidence in `resolve-backlogged-task` runtime history by explaining and remediating the incomplete success record.
**Scope:** Inspect the persisted job/job-run artifacts and the job-run completion path, verify whether the earlier runtime fix fully covers this scenario, and define the safe cleanup/recovery action.
**Assumptions:** The malformed run predates the later completion fix, but that has not been verified against the current code and persisted state.
**Risks:** Treating a live regression as a one-off cleanup could let future scheduler runs corrupt history again; editing runtime state without understanding the cause could hide evidence.

## Task 1: Reconstruct the incident

**Files:**
- Review: `.orbit/jobs/runs/job-1773033062432167000/jrun-1773115107199060000.yaml`
- Review: `.orbit/jobs/jobs/job-1773033062432167000.yaml`
- Review: `.orbit/tasks/done/T20260310-042915-1773116955241830000/execution-summary.md`
- Review: `orbit-core/src/command/job.rs`

**Steps:**
1. Compare the malformed persisted run with the current completion flow to identify which terminal fields were skipped.
2. Confirm whether the run timing aligns with the pre-fix runtime bug or whether the current code could still produce the same state.
3. Capture the exact operator-visible symptoms and the safest recovery boundary for the paused job.

**Done When:**
- There is a clear incident narrative and a reasoned hypothesis for why the run was marked `success` without terminal metadata.

## Task 2: Define and verify remediation

**Files:**
- Modify: runtime/store files only if needed after investigation
- Test: focused runtime or CLI coverage around job-run completion persistence

**Steps:**
1. If the bug is still reproducible, add a regression test and implement the minimal code fix.
2. If the bug is historical-only, document and perform the safe cleanup path for the orphaned run/job state.
3. Verify the final state with `orbit job-run show --json jrun-1773115107199060000`, `orbit job show --json job-1773033062432167000`, and any targeted tests added during the investigation.

**Done When:**
- The malformed run is either explained and safely cleaned up or prevented by a verified code fix, and operators can trust the backlog job history again.

## Final Verification
- `orbit job-run show --json jrun-1773115107199060000`
- `orbit job show --json job-1773033062432167000`
- Targeted verification command(s) added during investigation