# Glossary: Task

This glossary covers Orbit-specific task artifact terms. Generic issue-tracker or version-control vocabulary is excluded unless Orbit gives the term a narrower meaning.

| Term | Meaning |
|------|---------|
| **Acceptance document** | `acceptance.md`, the Markdown source of truth for validation expectations. See [2_design.md §2.3](../2_design.md). |
| **Artifact manifest** | `artifacts/manifest.yaml`, the structured index of files stored under a task's `artifacts/` directory. See [2_design.md §2.5](../2_design.md). |
| **Bundle** | The directory and files that together represent one task. See [1_overview.md §2.1](../1_overview.md). |
| **Envelope** | `task.yaml`, the small structured metadata file in a task bundle. See [1_overview.md §2.2](../1_overview.md). |
| **Global task ID** | The proposed canonical `ORB-A0001` identity allocated by an explicit authority. See [2_design.md §3](../2_design.md). |
| **Legacy ID** | A prior task identifier, usually `T<YYYYMMDD>-<N>`, retained in `legacy_ids` for lookup and history. See [2_design.md §3.3](../2_design.md). |
| **Prose sidecar** | A Markdown file that stores long-form task content outside `task.yaml`. See [1_overview.md §2.3](../1_overview.md). |
| **Status-neutral directory** | The v2 layout where `.orbit/tasks/<task-id>/` does not encode lifecycle state in the path. See [4_decisions.md ADR-003](../4_decisions.md#adr-003--status-neutral-task-directories). |
| **Task event stream** | Append-only lifecycle and metadata rows stored in `events.jsonl`. See [1_overview.md §2.5](../1_overview.md). |
| **Typed relation** | A structured task-to-task link with an explicit relation type such as `blocks` or `supersedes`. See [1_overview.md §2.6](../1_overview.md). |
