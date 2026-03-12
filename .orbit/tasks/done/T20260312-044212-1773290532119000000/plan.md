# Approval Skill Instruction Clarification Plan

**Goal:** Make the approval workflow instructions unambiguous so agents always return commit intent when they approve code changes at the `review -> done` gate.
**Scope:** Approval-skill wording, any mirrored skill assets, and any nearby docs/examples that describe review approval commit behavior.
**Assumptions:** Orbit's intended rule is that accepted code changes should be committed by Orbit via `result.commit`, and the task artifact included in that commit is the approved task bundle in `.orbit/tasks/done/<task_id>/` rather than any pre-done task state.
**Risks:** Updating only one copy of the skill or leaving examples vague could preserve inconsistent agent behavior.

## Task 1: Tighten the rule in the approval skill

**Files:**
- Modify: `.orbit/skills/orbit-approve-task/SKILL.md`
- Modify: `orbit-core/assets/skills/orbit-approve-task/SKILL.md`

**Steps:**
1. Rewrite the commit guidance so it explicitly distinguishes proposal approval from review approval.
2. State that a `review approved` result for accepted code changes must include `result.commit`.
3. Spell out the expected `files` contents: changed repo files, the task bundle under `.orbit/tasks/done/<task_id>/`, and any relevant job-run artifacts.
4. Explicitly state that non-`done` task bundles such as `proposed`, `backlog`, `in-progress`, `review`, `blocked`, or `rejected` are not the task artifacts to commit for this workflow.
5. Remove or replace ambiguous language such as "if the workflow expects a commit" if it weakens the rule.

**Done When:**
- A reader of the skill cannot reasonably approve changed code without also returning commit intent.
- A reader of the skill cannot reasonably mistake a `backlog` or other non-`done` task bundle for the artifact to commit.
- Both skill copies say the same thing.

## Task 2: Align examples and nearby guidance

**Files:**
- Review/Modify as needed: `orbit-core/src/command/job.rs`
- Review/Modify as needed: any docs or examples that describe `result.commit`
- Review: `.orbit/tasks/done/T20260311-032749-1773199669893488000/task.yaml`
- Review: `.orbit/tasks/done/T20260312-042623-1773289583885207000/task.yaml`

**Steps:**
1. Check whether surrounding docs/examples reinforce the same approval-time commit rule.
2. Update any mismatched example text so it matches the skill.
3. Keep the wording operational and specific about what artifacts belong in the commit.

**Done When:**
- The approval workflow docs consistently describe the same commit requirement.
- Examples consistently point at `.orbit/tasks/done/<task_id>/` for approved-task artifacts.