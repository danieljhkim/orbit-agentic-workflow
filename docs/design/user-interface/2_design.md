# User Interface — Design

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-05-07

This document describes the current Orbit UI implementation: the local dashboard assets, the CLI web API surface behind them, the Canon Refined visual rules they rely on [T20260427-29], and the telemetry behaviors that must stay consistent with backend data. The implementation is deliberately small: runtime state is rendered by static dashboard assets rather than a compiled frontend application [T20260506-20].

## 1. Runtime Shell

The dashboard lives in `crates/orbit-cli/assets/dashboard/` as `index.html` plus `app.js`, with the CLI web server exposing `/api/...` routes from `crates/orbit-cli/src/command/web/api.rs`. This keeps the operator console dependency-light and easy to ship with the binary, but it also means view structure, state transitions, and rendering helpers are hand-maintained in plain JavaScript.

Dashboard state is fetched from task YAML, job-run state, audit stores, scoreboard summaries, and diagnostics endpoints. The UI should preserve that read-mostly posture: task lifecycle actions are present, but durable workflow truth remains in Orbit stores, not browser-local state.

## 2. Dense Layout

The dashboard favors wide, dense tables and panels over narrative screens. Tight spacing, small radii, grouped lifecycle sections, and expandable sunken detail rows preserve hierarchy without hiding root lists.

The Tasks view pairs a grouped task table with a live log tail so an operator can correlate lifecycle state with recent execution noise. The Scoreboard view compresses companion metrics into pairs: `tokens` is `total/output`, `tool fail/all` is failed over total tool calls, and `duel w/all` is wins over participated duels. The primary friction column remains reported count only [T20260428-15].

## 3. Layered Palette and Typography

The UI uses layered dark surfaces instead of flat black: base canvas, elevated panels, sunken wells, and accent washes. Status color should stay muted and distinct; exact token values live in `./specs/theme.md` and the dashboard CSS.

`Inter` carries labels, headings, and prose. `JetBrains Mono` is reserved for IDs, metrics, timestamps, code, and log streams so numeric and diagnostic data stays aligned.

## 4. Live Status

Live processing is visible through pulsing dots, spinners, buffered-log counters, periodically refreshed tiles, and compact ticker-style values. The `orbit.log` panel is viewport-bounded; overflowing rows scroll inside the log stream so footer filters and follow-tail controls remain visible [T20260430-29]. Motion is functional: it points to active work without making the operator read raw logs first.

## 5. Audit and Policy Surfaces

Summary tiles and drill-down panels must agree. Audit > Policy is the detail view for the Denials 24h tile, so `/api/diagnostics/denials` combines v2 loop JSONL denial rows with SQLite `status = denied` audit events. SQLite filesystem boundary denials without an activity fsProfile use the stable `workspace-boundary` label [T20260428-13].

Audit Events filters by status, tool, role, execution id, profile, search text, and time window. Run Detail follows job-run ids and v2 audit envelope steps rather than pretending SQLite command audit rows contain the full activity/job trace. When a control links between these views, it should preserve the underlying identifier semantics instead of overloading labels such as `run_id`.

## 6. Diagnostics and Scoreboards

Diagnostics surfaces expose recent run metrics and friction rows so agent failures can be triaged without scraping process stdout. The scoreboard joins metrics-derived per-agent summaries with friction counts, duel participation, token/tool-call ratios, and audit denials by role.

Scoreboard cells should stay comparable at a glance. If a metric needs explanatory depth, the primary table should link or filter into a detail surface rather than adding more columns.

## 7. Concerns & Honest Limitations

Accessibility still needs a real WCAG pass; responsive behavior remains optimized for wide desktop viewports; raw HTML, CSS variables, and dashboard JavaScript keep the runtime simple but leave duplication across project surfaces.

The UI design folder is now convention-shaped, but the feature itself is still under-developed outside the local dashboard: there is no accepted component packaging strategy, no keyboard-interaction contract, no public-site reuse mechanism, and no automated visual regression suite [T20260506-20].

## Task References

- [T20260427-29] introduced the Canon Refined UI direction.
- [T20260428-13] unified dashboard denial sources for the policy drill-down.
- [T20260428-15] compacted scoreboard ratio columns.
- [T20260430-29] bounded the live `orbit.log` panel to the viewport.
- [T20260506-20] added required reference docs and clarified current UI scope.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
