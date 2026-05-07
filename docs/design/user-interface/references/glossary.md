# Glossary: User Interface

This glossary covers Orbit-specific UI vocabulary used by the User Interface docs. It deliberately excludes generic web, accessibility, and observability terms unless the Orbit dashboard gives them a narrower meaning.

| Term | Meaning |
|------|---------|
| **Audit > Policy** | The dashboard drill-down surface for policy denials, backed by `/api/diagnostics/denials` rather than the generic audit-events table alone. See [../2_design.md §5](../2_design.md#5-audit-and-policy-surfaces). |
| **Canon Refined** | Orbit's dense, layered dark UI language: compact spacing, subtle radii, muted semantic color, `Inter` for prose, and `JetBrains Mono` for diagnostic data. See [../2_design.md §3](../2_design.md#3-layered-palette-and-typography) and [../specs/theme.md](../specs/theme.md). |
| **Denials 24h** | The dashboard summary tile that counts recent policy denials from both SQLite command audit rows and v2 loop denial envelopes. See [../2_design.md §5](../2_design.md#5-audit-and-policy-surfaces). |
| **Live log tail** | The bounded `orbit.log` stream shown beside the Tasks view, with follow-tail and buffered-row controls kept visible while rows scroll internally. See [../2_design.md §4](../2_design.md#4-live-status). |
| **Scoreboard ratio** | A compact paired metric in the agent scoreboard, such as `tokens` as `total/output`, `tool fail/all`, or `duel w/all`. See [../2_design.md §6](../2_design.md#6-diagnostics-and-scoreboards). |
| **Workspace boundary** | The stable UI label for SQLite filesystem boundary denials that do not have an activity fsProfile. See [../2_design.md §5](../2_design.md#5-audit-and-policy-surfaces). |
