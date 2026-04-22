# Groundhog

**Status:** Draft
**Owner:** codex
**Last updated:** 2026-04-22

This folder is the canonical design home for Groundhog in Orbit. It is organized so the current implementation, the intended direction, and the remaining gaps do not blur together.

---

## 1. Read Order

1. [1_overview.md](./1_overview.md) — feature summary, motivation, and code map.
2. [2_design.md](./2_design.md) — the current implementation contract as it exists in the codebase today.
3. [implementation_status.md](./implementation_status.md) — the living gap tracker for what remains to implement or clean up.
4. [3_vision.md](./3_vision.md) — forward-looking questions and possible next moves.
5. [4_decisions.md](./4_decisions.md) — ADR log for Groundhog-specific architecture decisions.
6. [specs/](./specs) — narrow implementation-adjacent specs for specific subsystems.
7. [references/](./references) — glossary and supporting material.

---

## 2. Maintenance Rules

1. `2_design.md` describes shipped behavior, even when that behavior is awkward.
2. `implementation_status.md` is the place to record drift, missing work, and the last time each area was checked.
3. `3_vision.md` holds hypotheses and design pressure, not promises about what already exists.
4. `4_decisions.md` records durable architecture choices; append new ADRs instead of rewriting old ones away.
5. `specs/` should stay narrow and implementation-facing. If a spec becomes broad or philosophical, it probably belongs in `2_design.md` or `3_vision.md` instead.

---

## 3. Update Discipline

When Groundhog code changes:

- update `2_design.md` if runtime behavior, persistence, or tool surface changed
- update `implementation_status.md` if the change closed or created a gap
- add or extend an ADR in `4_decisions.md` if the change reflects a real architecture choice
- touch a focused file in `specs/` when a subsystem contract changed materially

Groundhog docs should be opinionated and trustworthy. If code and docs disagree, prefer fixing the docs immediately and then decide whether the code or the design needs to move.
