# User Interface — Vision

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-05-07

This document scopes forward-looking UI work beyond the current dashboard: richer interactivity, careful motion, and a reusable design system that can span local runtime views and public project pages.

## 1. Open Questions

1. **Framework adoption:** When dashboard state outgrows vanilla HTML/JS, is Preact, Svelte, or another small runtime worth the build complexity?
2. **Motion:** Can the mark or status surfaces respond to real telemetry without becoming decorative noise?
3. **Component sharing:** How do dashboard and static-site UI share tokens and components without a heavy Node.js pipeline?
4. **Interaction depth:** Which workflows deserve keyboard shortcuts, saved filters, or command-palette treatment once the dashboard becomes a daily operator surface?
5. **Accessibility target:** What WCAG level and assistive-technology matrix should Orbit commit to before the dashboard becomes more than a local developer tool?

## 2. Prior Work

### Terminal and TUI Tools
- **k9s and htop:** Dense, keyboard-driven system views that validate compact tables and high-contrast status.
- **Bloomberg Terminal:** A proof point for high-stakes density, fast recognition, and tolerated learning curves.

### Modern Pro-Tools
- **Linear and Vercel:** Benchmarks for quiet dark surfaces, precise typography, and keyboard-friendly interaction.

### Observability Consoles
- **Grafana and Honeycomb:** References for linking summary tiles into filtered detail surfaces while preserving source identifiers.
- **GitHub Actions:** A practical comparison point for run lists, expandable logs, retry/cancel actions, and audit-friendly timestamps.

## 3. What May Be Distinctive

Orbit can be distinctive by showing agent work as inspectable operations rather than hiding it behind chat. Graphs, traces, policy denials, scoreboards, and live logs should stay visible enough for engineering judgment while Canon Refined keeps the surface controlled [T20260427-29].

The vision is intentionally incremental at this stage. Near-term work should deepen the existing dashboard before inventing new surfaces: better run inspection, clearer state provenance, keyboardable dense tables, saved filters, and visual consistency between docs, dashboard, and project pages [T20260506-20].

## 4. References

- Orbit-internal: [Canon Refined Theme Spec](./specs/theme.md)
- Orbit-internal: [User Interface Glossary](./references/glossary.md)
- External: k9s, htop, Bloomberg Terminal, Linear, Vercel, Grafana, Honeycomb, and GitHub Actions remain comparison points for density, polish, traceability, and operator trust.

## Task References

- [T20260427-29] introduced the Canon Refined UI direction.
- [T20260506-20] documented the current minimal UI scope and reference vocabulary.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
