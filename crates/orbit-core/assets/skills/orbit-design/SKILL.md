---
name: orbit-design
description: Use this when scaffolding, listing, or inspecting design-doc feature folders under `docs/design/<feature>/` via `orbit.design.*`. Covers the four-numbered-doc layout, the `Last updated:` author assertion, the ADR earning rule, and why never to hand-create design folders or edit `Last updated:` to fake freshness.
---

# Orbit Design

## Purpose

Manage the design-doc system through the registered tool surface. Design docs are Orbit's convention for capturing architectural intent: every load-bearing feature gets a folder under `docs/design/<feature>/` with four numbered docs (overview, design, vision, decisions) plus `specs/` and `references/`. The convention rulebook lives in [docs/design/CONVENTIONS.md](../../../docs/design/CONVENTIONS.md); the meta design folder for the design-docs system itself is [docs/design/design-docs/](../../../docs/design/design-docs/).

Use this skill when: starting a new feature folder, auditing an existing folder before review, or pulling the on-disk state of any folder programmatically. Do **not** use it for prose authoring inside a doc — use file edits for that, then inspect the folder and review against [CONVENTIONS.md](../../../docs/design/CONVENTIONS.md).

## Tool Invocation

Both surfaces accept the same JSON. Use the CLI form when shell access is available; use the MCP names when the Orbit plugin exposes them.

| Tool | MCP | CLI |
|------|-----|-----|
| `orbit.design.init` | `orbit_design_init({...})` | `orbit tool run orbit.design.init --input '{"feature":"my-feature","owner":"claude"}'` |
| `orbit.design.list` | `orbit_design_list({...})` | `orbit tool run orbit.design.list --input '{}'` |
| `orbit.design.show` | `orbit_design_show({...})` | `orbit tool run orbit.design.show --input '{"feature":"my-feature"}'` |

Mapping rule: `orbit.design.<verb>` ↔ `orbit_design_<verb>`. Always include `model` in JSON inputs when the tool accepts it; pass your agent family (`codex`, `claude`, `gemini`, or `grok`). The same operations are also available through the top-level `orbit design init|list|show` CLI.

## Workflow

1. **Inspect before scaffolding.** Before adding a new folder, list what exists:
   - `orbit tool run orbit.design.list --input '{}'` returns one record per non-`_`-prefixed folder under `docs/design/` with per-doc `decay_status`, `owner`, `last_updated`, and absolute paths.
   - `orbit tool run orbit.design.show --input '{"feature":"<feature>"}'` returns the same shape for one feature, with a typed `design_feature_not_found` error when absent.
   If a folder for a closely-related concern already exists, prefer extending it (specs/ entries, mechanism sections in `2_design.md`) over creating a new sibling — the convention's [CONVENTIONS.md §10](../../../docs/design/CONVENTIONS.md) anti-pattern table rejects "tiny features that should have lived in an existing folder."

2. **Scaffold via `init`.** `orbit tool run orbit.design.init --input '{"feature":"<lowercase-hyphenated>","owner":"<agent-family>"}'` creates the folder atomically: four numbered docs (`1_overview.md`, `2_design.md`, `3_vision.md`, `4_decisions.md`) with required frontmatter pre-populated (today's date, the supplied `owner`), plus empty `specs/` and `references/` subfolders. The owner is the canonical agent family (`codex`, `claude`, `gemini`, or `grok`). The tool refuses to clobber an existing folder — delete or move first if you really mean to re-init. Feature names must be lowercase, hyphenated, and path-safe; the validator rejects spaces and uppercase.

3. **Author the four docs.** Edit the scaffolded files directly. Required sections per doc role are listed in [CONVENTIONS.md §3](../../../docs/design/CONVENTIONS.md). Common gotcha: `init` writes the `## Task References` section into `4_decisions.md` *above* where ADRs go — move it to the bottom (after the last ADR) before authoring entries, since ADRs are append-only and Task References belongs at end-of-file per existing folder convention.

4. **Review freshness before handoff.** Read any touched doc end-to-end, update the prose that changed, and bump `Last updated:` only when the doc body was actually reviewed or changed. The same-PR update rule in `CLAUDE.md` is the quality gate.

5. **Re-show to confirm.** After edits, `orbit tool run orbit.design.show --input '{"feature":"<feature>"}'` returns each doc's parsed owner, date, and path. Use this as the post-edit sanity check before declaring the folder ready.

## Operating Rules

- **Never scaffold a feature folder by hand.** Drift between sibling folders is the failure mode the convention exists to prevent. Use `orbit.design.init` so the four-doc layout, frontmatter, and section skeletons are correct from byte one.
- **Never bump `Last updated:` without reading the doc end-to-end.** That field is an explicit author assertion (see [docs/design/design-docs/4_decisions.md ADR-002](../../../docs/design/design-docs/4_decisions.md)). Cosmetic-only PRs do not bump the date; the field is the freshness signal, not file mtime.
- **Folder names are lowercase, hyphenated, singular.** The tool validates this; do not work around with underscores or PascalCase.
- **`init` does not clobber.** If you ran it with the wrong feature name, delete the folder and re-run; do not patch in place.
- **Required sections are non-negotiable.** Every numbered doc needs the sections listed in [CONVENTIONS.md §3](../../../docs/design/CONVENTIONS.md). Reviewers reject folders missing them; future lint may enforce mechanically.
- **ADRs use the 3-of-3 earning rule.** A decision belongs in `4_decisions.md` only when it has a real alternative, a forward constraint, and a non-trivial cost. Use the `orbit-adr` skill for the ADR artifact surface; the `orbit.design.*` tools manage *folders*, not individual ADRs.
- **Allocate ADR IDs globally before adding the local heading.** New `## ADR-` headings in `4_decisions.md` must use the ID returned by `orbit.adr.add` — see the `orbit-adr` skill and [ADR-0153]. Hand-writing a heading with a made-up number (3-digit local or 4-digit fake-global) produces an orphan decision invisible to the store; this is the failure mode [ORB-00098] resolved.
- **`_archive/` is the retirement path.** When a feature is retired, move the folder under `docs/design/_archive/<feature>/` and annotate the first line of `1_overview.md`. Tools skip `_`-prefixed folders automatically.

## Minimal Commands

Scaffold a new feature folder:

```bash
orbit tool run orbit.design.init --input '{"feature":"my-feature","owner":"claude"}' --pretty
```

List every existing feature folder:

```bash
orbit tool run orbit.design.list --input '{}' --pretty
```

Show one feature's metadata:

```bash
orbit tool run orbit.design.show --input '{"feature":"knowledge-graph"}' --pretty
```

## Common Mistakes

| Mistake | Why it fails | Correct form |
|---------|--------------|--------------|
| Copying a sibling folder by hand to start a new feature | Drift between sibling folders compounds; missed required sections; wrong frontmatter dates | `orbit tool run orbit.design.init --input '{"feature":"...","owner":"<agent-family>"}'` |
| Bumping `Last updated:` without re-reading the doc | The field is an explicit author assertion; lying breaks the freshness signal | Read the doc end-to-end, then bump |
| Adding a fifth top-level file (`README.md`, `roadmap.md`) under a feature folder | Anti-pattern table in [CONVENTIONS.md §10](../../../docs/design/CONVENTIONS.md) rejects it; readers expect exactly four numbered docs | Fold the content into the appropriate numbered doc, or into a `specs/` entry |
| Running `init` then re-running it after editing the wrong feature name | `init` refuses to clobber existing folders | Delete or move the wrong folder first, then re-run |
| Adding a `## ADR-` heading to `4_decisions.md` without calling `orbit.adr.add` first | Produces an orphan decision the global store does not know about; the heading number either invents a local sequence or collides with a real global ID | Allocate via `orbit.adr.add` first, then use the returned `ADR-NNNN` as the heading verbatim — see the `orbit-adr` skill and [ADR-0153] |

## Exit Criteria

The feature folder exists with the four numbered docs and two subfolders, each numbered doc has valid frontmatter (`Status`, `Owner`, `Last updated:`), and `orbit tool run orbit.design.show --input '{"feature":"<feature>"}'` reports the expected docs and paths.
