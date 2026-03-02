---
name: orbit-maintain-system
description: Perform routine, low-risk maintenance to keep the system healthy, consistent, and up-to-date without changing intended behavior. Use this skill only when explicitly requested. 
---

# Orbit Maintain System

Use this skill for routine maintenance that is safe, incremental, and non-disruptive.

---

## Inputs
- Repository workspace (cwd)
- Maintenance policy or schedule
- Optional scope constraints (paths, components)

---

## Responsibilities
1. Assess system health and identify maintenance issues.
2. Track every discovered issue via a new Orbit task.
3. Apply low-risk maintenance (deps, formatting, dead code, minor upgrades).
4. Verify integrity after changes (build/tests/lint as applicable).
5. Persist a markdown maintenance summary report.

---

## Assessment And Issue Tracking Contract

When assessment finds any issue (bug, risk, drift, failing check, deprecated usage, security concern), create a tracking task immediately via `orbit task add`.

Create one task per issue with:
- `--title` concise issue statement
- `--description` impact and observed evidence
- `--instructions` recommended fix or mitigation
- `--workspace` current repository absolute path
- Attribution: `--assigned-to`, `--created-by`, and `--identity` when an identity id is available
- `--type issue` (or `task` when clearly non-defect maintenance work)
- `--priority` mapped from severity (`high -> high`, `medium -> medium`, `low -> low`)

Attribution rule:
- If identity is available, use identity id + display name for attribution.
- If identity is not available, use model name fallback for `--assigned-to` and `--created-by`.
- Set `--identity` only when an identity id exists (identity id or model-alias identity).

Example:

```bash
orbit task add \
  --title "Fix failing maintenance check: rustfmt drift in orbit-core" \
  --description "Assessment detected formatting drift in orbit-core/src/...; CI risk: medium." \
  --instructions "Run rustfmt on affected files and re-run cargo test." \
  --workspace "/abs/path/to/repo" \
  --identity "linus" \
  --assigned-to "Linus Torvalds (Maintainer)" \
  --created-by "Linus Torvalds (Maintainer)" \
  --type issue \
  --priority medium
```

Record created task IDs in the final report.

If no issues are found, explicitly state "No maintenance issues found" in the report.

---

## Execution Contract
- Preserve observable behavior
- Prefer minimal, incremental changes
- Avoid breaking API or schema contracts
- Abort on validation failure
- Produce reviewable diffs
- Do not silently ignore discovered issues; track them with Orbit tasks

---

## Output

Persist a markdown report to:

`{{ORBIT_ROOT}}/agents/reports/YYYY-MM-DD-<title>.md`

The report must include:
- Maintenance request summary
- Scope and assessment timestamp (ISO date)
- Actions performed
- Files modified
- Validation results (`build/tests/lint`: pass|fail|skipped)
- Issues found
- Orbit tasks created for issues (task ID + title + priority + status)
- Follow-ups and risks

Return a concise markdown response that includes the report path and a short status summary.

---

## Report Template

```markdown
# Maintenance Summary - <Title>

## Status
success | failed

## Scope
<paths/components reviewed>

## Actions Performed
- <action>

## Files Modified
- <file path>

## Validation
- Build: pass | fail | skipped
- Tests: pass | fail | skipped
- Lint: pass | fail | skipped

## Issues Found
- <issue summary>

## Orbit Tasks Created
- <TASK_ID> - <title> (priority: <low|medium|high>, status: <todo|in-progress|done>)

## Notes
<follow-ups, risks, blockers>
```

---

## Exit Criteria
- Assessment completed
- Every discovered issue is tracked via newly created Orbit task(s)
- Maintenance actions completed or safely skipped
- Validation completed
- Markdown report written to `{{ORBIT_ROOT}}/agents/reports/YYYY-MM-DD-<title>.md`
- Response includes report location and outcome summary
