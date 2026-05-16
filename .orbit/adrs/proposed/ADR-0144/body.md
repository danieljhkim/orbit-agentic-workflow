## Context
`task.yaml` currently stores metadata, long prose, acceptance criteria, comments, history, and review threads together. This makes simple tasks easy to inspect, but it turns every content edit or append into a YAML rewrite and makes Markdown-hostile fields harder for humans and agents to author.

## Decision
Keep `task.yaml` as a small structured envelope and move prose into Markdown sidecars: `description.md`, `acceptance.md`, `plan.md`, and `execution-summary.md`. Public APIs should treat those sidecars as first-class task documents rather than maintaining embedded-YAML compatibility. Because the reset is a new artifact family, the v2 envelope starts at `schema_version: 1` instead of continuing the old compatibility stream.

## Consequences
- Prose gets native Markdown editing, diffs, and rendering.
- YAML becomes smaller, easier to validate, and easier to merge.
- CLI/tool reads and writes must operate on document sidecars directly.
- Cutover code knows how to read the old schema, but v2 readers do not carry old embedded-field fallbacks.
- Cost: one task now spans more files. Scripts that read only `task.yaml` must switch to the bundle API.