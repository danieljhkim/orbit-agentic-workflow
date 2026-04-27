# User Interface — Decisions

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-04-26

This document records the architectural and design decisions for the Orbit User Interface. It is append-only.

## ADR-001 — Canon Refined Aesthetic

**Status:** Proposed · 2026-04 · [T20260427-29]

**Context.** The dashboard and website require a cohesive visual identity. The previous "Trading Terminal" aesthetic (pure monospace, sharp corners, harsh neon colors) proved too rigid and inaccessible for complex, hierarchical data. We need a design that maintains high density and a "pro-tool" feel while improving readability and structure.

**Decision.** We adopt the "Canon Refined" aesthetic (layered dark mode, dual typography using `Inter` and `JetBrains Mono`, soft semantic colors, and subtle border radii).

**Consequences.**
- We gain high data density and a modern, accessible "pro-tool" visual brand.
- We drop the strict adherence to retro constraints, allowing for standard web affordances.
- Cost: We must formalize a design system to prevent the aesthetic from drifting into generic "Web 2.0" styling, requiring more disciplined CSS architecture.

## Task References

- [T20260427-29]

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
