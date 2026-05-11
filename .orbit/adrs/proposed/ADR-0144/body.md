## Context
`task.yaml` currently stores metadata, long prose, acceptance criteria, comments, history, and review threads together. This makes simple tasks easy to inspect, but it turns every content edit or append into a YAML rewrite and makes Markdown-hostile fields harder for humans and agents to author.

## Decision
Keep `task.yaml` as a small structured envelope and move prose into Markdown sidecars: `description.md`, `acceptance.md`, `plan.md`, and `execution-summary.md`. Public APIs may expose logical string or list fields, but storage treats the files as source of truth.

## Consequences
- Prose gets native Markdown editing, diffs, and rendering.
- YAML becomes smaller, easier to validate, and easier to merge.
- Existing CLI and tool projections need a compatibility layer that reads sidecars.
- Cost: one task now spans more files. Simple scripts that read only `task.yaml` must switch to the bundle API.