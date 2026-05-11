## Context
The current `T<YYYYMMDD>-<N>` format is allocated by scanning one workspace task tree. That shape is useful as a local search key but fails when tasks need to be referenced across workspaces, shared registries, hosted Team, or durable design docs without explaining which machine allocated them.

## Decision
Adopt `ORB-00000` as the canonical v2 task ID format: `ORB-` plus a five-digit decimal suffix allocated by an explicit authority. Store bundles under 100-task partitions derived from the suffix (`floor(n / 100)`, zero-padded to three digits), for example `.orbit/tasks/000/ORB-00000/` and `.orbit/tasks/123/ORB-12345/`. V2 task bundles do not preserve old `T...` identifiers as aliases.

## Consequences
- Task IDs become meaningful across the scope of the configured allocator instead of implicitly workspace-local.
- Filesystem fanout is bounded without encoding lifecycle state in the path.
- Implementations must stop validating only `T<YYYYMMDD>-<N>` and must add numeric `ORB-\d{5}` validation and partition derivation.
- Existing local tasks need a cutover command, but the result is a clean v2 task store rather than a dual-ID store.
- Cost: task creation now depends on an allocator outside the task directory scan. Sync and hosted modes need online allocation before a task can be published.