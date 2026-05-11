# Glossary: Task

This glossary covers Orbit-specific task artifact terms. Generic issue-tracker or version-control vocabulary is excluded unless Orbit gives the term a narrower meaning.

| Term | Meaning |
|------|---------|
| **Acceptance document** | `acceptance.md`, the Markdown source of truth for validation expectations. See [2_design.md §2.3](../2_design.md). |
| **Artifact manifest** | `artifacts/manifest.yaml`, the structured index of files stored under a task's `artifacts/` directory. See [2_design.md §2.5](../2_design.md). |
| **Backup store** | The durable local task copy under `~/.orbit/tasks/`, used to allocate IDs, preserve workspace bindings, and restore repo-local task bundles. See [2_design.md §2.6](../2_design.md). |
| **Bundle** | The directory and files that together represent one task. See [1_overview.md §2.1](../1_overview.md). |
| **Envelope** | `task.yaml`, the small structured metadata file in a task bundle. See [1_overview.md §2.2](../1_overview.md). |
| **Global task ID** | The proposed canonical `ORB-00000` identity allocated by an explicit authority. See [2_design.md §3](../2_design.md). |
| **Materialized task bundle** | The workspace-local copy of a task under `.orbit/tasks/<partition>/<task-id>/`, rebuilt from the backup store when needed. See [4_decisions.md ADR-007](../4_decisions.md#adr-007--repo-local-task-bundles-backed-by-orbit). |
| **Prose sidecar** | A Markdown file that stores long-form task content outside `task.yaml`. See [1_overview.md §2.3](../1_overview.md). |
| **Status-neutral directory** | The v2 layout where `.orbit/tasks/<partition>/<task-id>/` does not encode lifecycle state in the path. See [4_decisions.md ADR-003](../4_decisions.md#adr-003--status-neutral-task-directories). |
| **Task event stream** | Append-only lifecycle and metadata rows stored in `events.jsonl`. See [1_overview.md §2.5](../1_overview.md). |
| **Typed relation** | A structured task-to-task link with an explicit relation type such as `blocks` or `supersedes`. See [1_overview.md §2.6](../1_overview.md). |
