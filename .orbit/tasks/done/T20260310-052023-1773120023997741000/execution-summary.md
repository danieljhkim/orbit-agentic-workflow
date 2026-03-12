# Execution Summary - Clarify task status naming to prevent in_progress CLI mistakes
Agent Name: Grace
Agent Model: GPT-5 Codex

## Status
success

## Orbit Task
Task ID: T20260310-052023-1773120023997741000

## 1. Summary of Changes
Updated task status handling so CLI-facing output now uses `in-progress` while still accepting the common `in_progress` alias on input.
Added regression coverage in `orbit-types` and the task CLI integration tests for alias parsing plus `show` and `list` output.
Revised the relevant shipped Orbit skills to teach `in-progress` in lifecycle guidance and to explain that the on-disk task bundle directory remains `in_progress`.

## 2. Strategic Decisions
- Keep storage and bundle-directory naming unchanged while normalizing CLI-facing output to `in-progress` | Rationale: this fixes the repeated human/agent mistake without creating a migration or breaking existing task bundles. | Trade-offs: Orbit now has an intentional distinction between internal storage spelling and external CLI spelling, which must stay documented.
- Accept both `in_progress` and `in-progress` in parsing | Rationale: existing agents already use the snake_case form, so aliasing removes friction immediately. | Trade-offs: two accepted spellings remain valid at input time, but only one is surfaced back to users.
- Update local `.orbit/skills` and built-in asset copies together | Rationale: both sources are used in practice, and leaving one side stale would keep reintroducing the mismatch. | Trade-offs: slightly larger edit surface for a focused UX fix.

## 3. Assumptions Made
- No user-facing README update was necessary because the relevant status guidance lives in the task skills and CLI behavior rather than the current top-level README content. | Impact if incorrect: we may need a follow-up docs update if another surfaced CLI guide still teaches `in_progress`.

## 4. Design Weaknesses / Risks
- Other user-facing surfaces outside the patched task commands and skills could still mention `in_progress`. | Severity: Medium | Mitigation: follow the same CLI-facing/internal-storage split in future status-related docs or commands.

## 5. Deviations from Original Plan
- Updated `orbit-execute-change-request` skill copies in addition to the originally listed skill files. | Justification: that skill also taught `in_progress`, so leaving it unchanged would keep the agent-facing guidance inconsistent.

## 6. Technical Debt Introduced
- None beyond the documented dual spelling policy for input versus storage. | Recommended resolution: if Orbit later standardizes all task lifecycle naming, introduce an explicit status-format helper policy and reuse it across docs and commands.

## 7. Recommended Follow-Ups
- When implementing the planned `rejected` status, apply the same rule: pick one CLI-facing spelling and document any internal storage differences in the skills at the same time.

## 8. Overall Assessment
The change is small but high leverage: it removes an active agent failure mode, keeps persistence stable, and adds regression coverage around the exact command behavior that was causing repeated friction.
