# Execution Summary - Default manual jobs are hidden from orbit job list
Agent Name: Grace
Agent Model: GPT-5 Codex

## Status
success

## Orbit Task
Task ID: T20260310-064640-1773125200538593000

## 1. Summary of Changes
Changed default named job seeding so the built-in manual jobs created by `orbit init` now initialize in `enabled` state, which makes them visible in the existing `orbit job list` output without changing the list filter itself.
Kept ordinary `orbit job add --schedule manual` behavior unchanged, so user-created manual jobs still start `disabled`.
Added regression coverage in init and job CLI tests to verify seeded default jobs are visible and enabled after init, and that seeded manual jobs can still be paused and resumed.

## 2. Strategic Decisions
- Add an internal `initial_state_override` to `JobAddParams` instead of broadening `job list` | Rationale: this preserves the existing operator expectation that plain `job list` shows active jobs while still surfacing the seeded defaults. | Trade-offs: introduces a small internal-only extension point in the add-job path.
- Keep seeded jobs on `schedule: manual` while marking them `enabled` | Rationale: matches your requested behavior and preserves explicit-run semantics because their `next_run_at` stays far in the future. | Trade-offs: manual jobs now have two supported initial states internally, so that policy must stay intentional.
- Special-case `resume_job` for manual schedules | Rationale: once seeded manual jobs start enabled, pause/resume needs to work cleanly for them as part of normal lifecycle management. | Trade-offs: another manual-schedule branch in the runtime, but it is small and well-covered.

## 3. Assumptions Made
- The default named jobs should remain manually triggered jobs rather than receiving real recurring schedules. | Impact if incorrect: a follow-up change should assign explicit schedules instead of relying on enabled manual jobs with far-future next-run timestamps.

## 4. Design Weaknesses / Risks
- Enabled manual jobs still rely on a far-future sentinel timestamp for scheduler bookkeeping. | Severity: Low | Mitigation: if manual jobs gain richer lifecycle semantics later, introduce a first-class unscheduled/manual next-run representation instead of the sentinel.
- Existing task/docs content elsewhere in the repo may still describe the seeded default jobs as disabled. | Severity: Medium | Mitigation: update those task artifacts or follow-up documentation if they are surfaced to users as authoritative guidance.

## 5. Deviations from Original Plan
- Adjusted `resume_job` in addition to default seeding. | Justification: enabling seeded manual jobs by default exposed a real lifecycle edge where paused manual jobs could not be resumed cleanly.

## 6. Technical Debt Introduced
- The general `JobAddParams` struct now carries an internal state override used by init seeding. | Recommended resolution: if more seeded or system-managed jobs need custom initialization, consider a dedicated internal job-provisioning helper instead of growing the public-ish add params further.

## 7. Recommended Follow-Ups
- Update any remaining default-job provisioning docs or task notes that still say the seeded jobs are `disabled` by default.
- Decide whether the seeded default jobs should eventually receive explicit schedules instead of staying manual but enabled.

## 8. Overall Assessment
This is a focused fix that aligns the product with the expected operator workflow: `orbit init` now creates visible default jobs without weakening the default `job list` filter, and the seeded manual jobs retain sane lifecycle behavior.
