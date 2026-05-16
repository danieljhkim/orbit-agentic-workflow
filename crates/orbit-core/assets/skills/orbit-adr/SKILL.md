---
name: orbit-adr
description: Use this when creating, updating, listing, showing, accepting, or superseding Orbit Architecture Decision Record artifacts via `orbit.adr.*`. Covers ADR body requirements, global ADR ID allocation, related_features/related_tasks/legacy_ids, lifecycle transitions, and how to avoid editing `.orbit/adrs/` files directly.
---

# Orbit ADR

## Purpose

Create and maintain Orbit ADR artifacts through the registered tool surface. ADRs record decisions, not implementation plans: use them when a choice has a real alternative, constrains future work, and carries a non-trivial cost.

## Tool Invocation

Both surfaces accept the same JSON. Use the CLI examples below when shell access is available; use the MCP names when the Orbit plugin exposes them.

| Tool | MCP | CLI |
|------|-----|-----|
| `orbit.adr.add` | `orbit_adr_add({...})` | `orbit tool run orbit.adr.add --input-file adr.json` |
| `orbit.adr.show` | `orbit_adr_show({...})` | `orbit tool run orbit.adr.show --input '{"id":"ADR-0042"}'` |
| `orbit.adr.list` | `orbit_adr_list({...})` | `orbit tool run orbit.adr.list --input '{"feature":"task-artifacts"}'` |
| `orbit.adr.update` | `orbit_adr_update({...})` | `orbit tool run orbit.adr.update --input-file update.json` |
| `orbit.adr.supersede` | `orbit_adr_supersede({...})` | `orbit tool run orbit.adr.supersede --input '{"old_id":"ADR-0041","new_id":"ADR-0042"}'` |

Always include `model` in JSON inputs when the tool accepts it. Prefer `--input-file` for `add` and body-changing `update` calls so markdown does not get mangled by shell quoting.

Run `orbit tool show orbit.adr.add` or `orbit tool list` instead of guessing if the local tool surface has changed. Do not assume future surfaces such as `orbit.adr.search` or `orbit.adr.review_thread.*` exist unless `orbit tool list` shows them.

## Workflow

1. Inspect nearby decisions before adding a new one.
   - `orbit tool run orbit.adr.list --input '{"feature":"<feature>","model":"<model_name>"}'`
   - `orbit tool run orbit.adr.show --input '{"id":"ADR-NNNN","model":"<model_name>"}'`
   - Use `legacy_id` lookup for migrated per-feature references, for example `{"legacy_id":"activity-job/ADR-039"}`.
2. Decide whether this is a new ADR, an update to a proposed ADR, or a supersession.
   - New decision: `orbit.adr.add`.
   - Body/metadata correction: `orbit.adr.update`.
   - Reversal or replacement of an accepted ADR: create the replacement, accept it with a related task, then `orbit.adr.supersede`.
3. Write the body with exactly the required sections: `## Context`, `## Decision`, `## Consequences`.
4. Include at least one consequences bullet starting with `Cost:`.
5. Set `related_features` to feature folder names such as `task-artifacts`, `activity-job`, or `policy-sandbox`.
6. Leave `related_tasks` empty for speculative proposed ADRs when no task exists yet. Do not create or invent a task just to satisfy an ADR proposal. Acceptance requires a real related task.
7. Verify with `orbit.adr.show` or `orbit.adr.list`.

## Creation Rules

- Never edit `.orbit/adrs/<status>/<id>/adr.yaml` or `body.md` directly.
- Never invent the global ADR ID. `orbit.adr.add` allocates it.
- Create an ADR only when all three are true:
  - A real alternative was on the table.
  - The choice constrains future work.
  - The cost is non-trivial and worth preserving for future readers.
- Put routine implementation detail in `2_design.md`, a spec file, or an existing ADR's instance table instead of creating a new ADR.
- `related_tasks` may be empty at creation. `proposed -> accepted` requires at least one real task ID on the resulting record.
- Use `legacy_ids` only for aliases that already mean something, such as migrated `feature/ADR-NNN` entries or a local per-feature doc entry you just created in the same change.

## Body Template

```markdown
## Context
<1-3 sentences. What forced a decision and what alternatives were real?>

## Decision
<1-3 sentences. What was chosen?>

## Consequences
- <observable or operational consequence>
- <another consequence>
- Cost: <explicit tradeoff that future readers need to know>
```

Keep bodies concise. ADRs are durable memory for the decision, not a full project brief.

## Minimal Commands

Create a proposed ADR:

`/tmp/orbit-adr.json`:

```json
{
  "title": "Short noun phrase",
  "body": "## Context\nWhat forced the decision.\n\n## Decision\nWhat was chosen.\n\n## Consequences\n- What improves or changes.\n- Cost: The real tradeoff.\n",
  "owner": "codex",
  "related_features": ["task-artifacts"],
  "related_tasks": [],
  "model": "<model_name>"
}
```

```bash
orbit tool run orbit.adr.add --input-file /tmp/orbit-adr.json --pretty
```

Attach a legacy per-feature alias after creation:

```bash
orbit tool run orbit.adr.update --input '{
  "id": "ADR-0143",
  "legacy_ids": ["task-artifacts/ADR-001"],
  "model": "<model_name>"
}'
```

Accept a proposed ADR only once a real task exists:

```bash
orbit tool run orbit.adr.update --input '{
  "id": "ADR-0143",
  "status": "accepted",
  "related_tasks": ["T20260510-28"],
  "model": "<model_name>"
}'
```

Supersede an old ADR:

```bash
orbit tool run orbit.adr.supersede --input '{
  "old_id": "ADR-0041",
  "new_id": "ADR-0042",
  "model": "<model_name>"
}'
```

## Common Mistakes

| Mistake | Why it fails | Correct form |
|---------|--------------|--------------|
| Hand-writing `.orbit/adrs/...` files | Skips allocation, validation, indexes, and provenance | Use `orbit.adr.add` or `orbit.adr.update` |
| Creating a task just to propose an ADR | Proposed ADRs explicitly allow empty `related_tasks` | Leave `related_tasks: []` until acceptance |
| Accepting an ADR without a real task | Lifecycle requires implementation linkage at acceptance | Add the task ID in the same `orbit.adr.update` call |
| Omitting `Cost:` | The validator rejects new ADRs without an explicit cost | Include a consequences bullet beginning `Cost:` |
| Treating local `ADR-001` as global | Local per-feature numbers are legacy aliases | Use global `ADR-NNNN` and optional `legacy_ids` |

## Exit Criteria

The ADR artifact exists or is updated through `orbit.adr.*`, has a valid body, names the relevant features, preserves any meaningful legacy aliases, and can be read back with `orbit.adr.show`.
