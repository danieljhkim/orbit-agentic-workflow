# Orbit Skills Refactor Plan

**Goal:** Two-layer skill architecture — thin reference + trimmed standalone skills; enforce tool call convention via activity skillRefs.
**Scope:** `.orbit/skills/` directory and activity instruction boilerplate. Activity logic is out of scope.
**Assumptions:** Activities are the authoritative source for workflow steps; skills should not duplicate them. Agents invoke orbit via tool calls, not shell CLI.
**Risks:** Removing too much from skills could leave standalone agents under-informed. Removing 1% rule (done) may under-load skills in edge cases — mitigated by explicit skillRefs in activities.

**Already done (pre-task):**
- `orbit-operations-management` deleted and merged into `orbit-maintain-system`.
- 1% skill activation rule removed from global `~/.claude/CLAUDE.md`.

## Task 1: Audit existing skills vs activity instructions

**Files:**
- Read: `.orbit/skills/*/SKILL.md` (all remaining skills)
- Read: `.orbit/activities/active/*.yaml` (all active activities, focus on `instruction` field)

**Steps:**
1. For each skill, list sections duplicated in at least one activity instruction.
2. List sections only in skills (no activity covers them) — these must be preserved.
3. Produce a written audit table: skill | duplicated sections | unique sections.

**Done When:**
- Audit table exists and reviewed before any file changes.

## Task 2: Delete redundant skills

**Files:**
- Delete: `.orbit/skills/orbit-assess-codebase/` (redundant — covered by activity instructions)
- Delete: `.orbit/skills/orbit-manage-tasks/` (redundant — covered by activity instructions)
- Delete: `.orbit/skills/orbit-skills/` (renamed — see Task 4)
- Remove corresponding symlinks under `.claude/skills/`

**Done When:**
- Directories and symlinks removed. Verify no active activity or job YAML references them (grep `.orbit/activities/` and `.orbit/jobs/`).

## Task 3: Create `orbit-reference` skill

**Files:**
- Create: `.orbit/skills/orbit-reference/SKILL.md`
- Create: `.claude/skills/orbit-reference` symlink → `.orbit/skills/orbit-reference`

**Content to include (and only this):**
- Lifecycle state diagram (`proposed → backlog → in-progress → review → done`, rejection path)
- Orbit tool call reference (NOT shell CLI): `orbit.task.list`, `orbit.task.show`, `orbit.task.update`, `orbit.task.add`, `orbit.job.run`, `orbit.identity.list` — parameters and return shapes
- Skill index: one-line description of each remaining skill and when to invoke it
- Note: agents interact with orbit via tool calls, not shell commands; only load skills listed in the activity's skillRefs

**Done When:**
- `orbit-reference/SKILL.md` exists, is under 80 lines, covers all three content areas.

## Task 4: Rename `orbit-skills` → `orbit`

**Files:**
- Create: `.orbit/skills/orbit/SKILL.md`
- Create: `.claude/skills/orbit` symlink → `.orbit/skills/orbit`
- Delete: `.orbit/skills/orbit-skills/` and its symlink

**Steps:**
1. Base content on `orbit-skills/SKILL.md`.
2. Update frontmatter: `name: orbit`.
3. Update skill index to reflect all deletions and the new `orbit-reference` skill.
4. Explain the two-layer architecture: activity = workflow, `orbit-reference` = facts, full skills = standalone orientation.
5. Add note: agents should only load skills listed in their activity's `skillRefs`; standalone agents without an activity use this skill to orient and pick the right one.

**Done When:**
- `.orbit/skills/orbit/SKILL.md` exists with correct frontmatter and updated skill index.

## Task 5: Enforce skillRefs convention in activity instructions

**Files:**
- Modify: `.orbit/activities/active/*.yaml` (all active activities)

**Steps:**
1. For each activity, add a `skillRefs` field listing only the skills that activity actually needs (if not already present).
2. Add a standard preamble line to each activity's `instruction` field: "Only use skills listed in this activity's skillRefs. Ignore all others."
3. Verify no activity references deleted skills (orbit-assess-codebase, orbit-manage-tasks, orbit-operations-management).

**Done When:**
- All active activities have explicit `skillRefs`.
- All activity instructions include the ignore-other-skills preamble.

## Task 6: Trim duplicated content from remaining skills

**Files:**
- Modify: `.orbit/skills/orbit-*/SKILL.md` (each skill with duplication per audit)

**Steps:**
1. Remove sections duplicated in activity instructions (per Task 1 audit).
2. Add at the top: "For CLI reference and lifecycle overview, see `orbit-reference`."
3. Replace any shell CLI examples (`orbit task ...`) with tool call equivalents (`orbit.task.*`).
4. Keep: decision heuristics, safety rules, output requirements, verification steps unique to the skill.

**Done When:**
- No remaining skill duplicates lifecycle diagram or tool reference in `orbit-reference`.
- No skill uses shell CLI syntax for orbit operations.
- Each trimmed skill still makes sense as a standalone document.

## Final Verification
- All remaining skills have correct frontmatter and load cleanly.
- `orbit-reference` is ≤ 80 lines.
- A standalone agent reading only `orbit` can identify the right skill to load.
- A standalone agent reading only a trimmed skill can complete its workflow using tool calls.
- No references to deleted skills remain in any active artifact.
- All active activities have `skillRefs` and the ignore-other-skills preamble.