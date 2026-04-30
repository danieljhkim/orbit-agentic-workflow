# User Interface — Decisions

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-04-30

This append-only ADR log records UI decisions in ascending order. Each entry keeps its status line, cited task ID, decision summary, and at least one explicit cost.

## ADR-001 — Canon Refined Aesthetic

**Status:** Proposed · 2026-04 · [T20260427-29]

**Context.** The dashboard and project website need one visual identity. The prior Trading Terminal direction was dense but too rigid for hierarchical data, review threads, and mixed telemetry.

**Decision.** Adopt Canon Refined: layered dark surfaces, `Inter` plus `JetBrains Mono`, soft semantic colors, compact spacing, and subtle radii.

**Consequences.**
- The UI keeps a serious pro-tool signal while allowing standard web affordances when they improve operator clarity.
- Cost: The design system must be maintained so Canon Refined does not drift into generic dark SaaS styling.

## ADR-002 — Unified Denial Sources for Policy Dashboard

**Status:** Accepted · 2026-04 · [T20260428-13]

**Context.** The Denials 24h tile counted SQLite audit rows and v2 loop denials, but the Policy tab originally scanned only v2 loop JSONL files. Direct CLI denials could increment the tile while the detail table appeared empty.

**Decision.** Aggregate v2 denial envelopes and SQLite `status = denied` audit events in the policy-denials endpoint. SQLite filesystem denials without an activity fsProfile use `workspace-boundary`.

**Consequences.**
- Audit > Policy is a faithful drill-down for Denials 24h, including direct `orbit tool run` policy denials.
- Cost: The endpoint carries a translation layer because SQLite audit rows lack typed denial fields like `profile` and `path`.

## ADR-003 — Compact Scoreboard Ratio Columns

**Status:** Accepted · 2026-04 · [T20260428-15]

**Context.** The scoreboard had separate columns for output tokens, tool calls, duel wins/losses, and friction triage. After failed tool calls became first-class, the split counters made reliability harder to scan.

**Decision.** Render companion metrics as compact pairs: `tokens` is `total/output`, `tool fail/all` is failed over all tool calls, and `duel w/all` is wins over participated duels. Keep only friction reports in the primary table.

**Consequences.**
- The table presents reliability and participation context in fewer columns, while `0/N` tool failures stays meaningful.
- Cost: Friction accepted/rejected counts and raw duel losses require summary JSON or a future detail view.

## ADR-004 — Bounded Live Log Tail

**Status:** Accepted · 2026-04 · [T20260430-29]

**Context.** The Tasks view keeps `orbit.log` visible beside the task list, but the log panel could grow taller than short viewports and push footer controls below the screen.

**Decision.** Keep the Tasks view in a two-column layout and size `#log-panel` to the available viewport. The log row stream owns overflow scrolling, while filters, buffered-count, and follow-tail controls remain inside the bounded panel.

**Consequences.**
- Operators get one clear scroll target for raw log rows while live-tail controls stay visible during short-screen monitoring.
- Cost: The Tasks view trades narrow-screen stacking for denser columns so the live log remains in the first viewport.

## Task References

- [T20260427-29] introduced the Canon Refined UI direction.
- [T20260428-13] unified policy-denial sources for the dashboard.
- [T20260428-15] compacted scoreboard ratio columns.
- [T20260430-24] tightened this ADR log without changing decisions.
- [T20260430-29] bounded the live `orbit.log` tail panel.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
