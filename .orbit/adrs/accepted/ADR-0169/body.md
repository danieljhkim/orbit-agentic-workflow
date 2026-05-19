## Context
The Scoreboard tab rendered six separate tables stacked vertically, making cross-table comparison difficult. We needed a layout that allowed at-a-glance cross-agent comparison without excessive vertical scroll or visual clutter from zero values.

Rejected Alternative 1: Separate cards per metric. Rejected because comparing all metrics for a single agent requires jumping across 6 cards, defeating the cross-metric comparison requirement.
Rejected Alternative 2: Dropdown to select agent. Rejected because hiding agents prevents at-a-glance comparison.

## Decision
Replace the stacked tables with a single agent-major Matrix Grid layout. Agents are rows, metrics are columns grouped under colspan category headers. Zero values render as an empty dot, and relative performance is encoded with a CSS `perf-bar` proportional to the column max.

## Consequences
- A reader can identify the leading agent for any metric via visual encoding without reading digits.
- Cost: Calculating maximums per column for the performance bars requires an extra pass over the data before rendering the rows.