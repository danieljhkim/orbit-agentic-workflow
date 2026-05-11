## Context
Task links currently appear as `parent_id`, `dependencies`, `source_task_id`, `batch_id`, and external references. Each field has its own semantics, and adding a new relation requires another schema field or prose convention.

## Decision
Use a `relations` array with explicit relation types for task-to-task links. The v2 envelope stores one typed relation surface and does not retain `parent_id`, `dependencies`, or `source_task_id` as compatibility fields.

## Consequences
- Consumers can traverse relationships by meaning instead of hardcoded field names.
- Future relation types can be added without widening the top-level envelope.
- Task lineage can share vocabulary with the task artifact rather than deriving every edge from prose.
- Cost: relation validation becomes stricter and more complex. Existing callers that set old link fields must be updated to write the typed relation surface.