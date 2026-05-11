## Context
Task bundles need to be close to the workspace so agents can inspect and update them with project context, but keeping the only copy under the repository checkout makes accidental deletion catastrophic and makes Git tracking pressure worse. The alternatives are committing tasks, storing tasks only in a global home directory, or using a workspace materialization backed by a durable local store.

## Decision
Treat `.orbit/tasks/<partition>/<task-id>/` as the workspace-local materialized working copy and store a durable backup copy under `~/.orbit/tasks/<partition>/<task-id>/`, plus allocator, checksum, and workspace-binding indexes under `~/.orbit/tasks/`. Local task writes update the backup layer and the workspace materialization as one logical operation; deleting `.orbit/tasks/` should be recoverable from the home-directory backup.

## Consequences
- Task artifacts remain available next to the code without becoming committed source artifacts.
- Local-only Orbit gets a recovery path when `.orbit/tasks/` is deleted or a checkout is recreated.
- Sync and hosted modes can replace or augment the local backup authority without changing the workspace bundle shape.
- Recovery must detect divergence between the workspace materialization and backup before overwriting envelopes or Markdown documents.
- Cost: task writes now need backup/index maintenance and recovery conflict rules when the workspace copy and home-directory copy diverge.