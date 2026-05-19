const fs = require('fs');
const docPath = 'docs/design/user-interface/4_decisions.md';
let doc = fs.readFileSync(docPath, 'utf8');

doc = doc.replace(/last_updated: 2026-05-18/, 'last_updated: 2026-05-19');

const adrText = `

## ADR-0169 — Unified Matrix Grid Scoreboard (Agent-Major)

**Status:** Accepted · 2026-05 · [ORB-00155]

**Context.** The Scoreboard tab rendered six separate tables stacked vertically, making cross-table comparison difficult. We needed a layout that allowed at-a-glance cross-agent comparison without excessive vertical scroll or visual clutter from zero values.

Rejected Alternative 1: Separate cards per metric. Rejected because comparing all metrics for a single agent requires jumping across 6 cards, defeating the cross-metric comparison requirement.
Rejected Alternative 2: Dropdown to select agent. Rejected because hiding agents prevents at-a-glance comparison.

**Decision.** Replace the stacked tables with a single agent-major Matrix Grid layout. Agents are rows, metrics are columns grouped under colspan category headers. Zero values render as an empty dot, and relative performance is encoded with a CSS \`perf-bar\` proportional to the column max.

**Consequences.**
- A reader can identify the leading agent for any metric via visual encoding without reading digits.
- Cost: Calculating maximums per column for the performance bars requires an extra pass over the data before rendering the rows.
`;

doc = doc.replace('## Task References', adrText + '\n## Task References');
doc = doc.replace('- [ORB-00154] unified the Scoreboard tab into a metric-major leaderboard matrix.', '- [ORB-00154] unified the Scoreboard tab into a metric-major leaderboard matrix.\n- [ORB-00155] unified the Scoreboard tab into an agent-major leaderboard matrix.');

fs.writeFileSync(docPath, doc);
