## Context
The current `T<YYYYMMDD>-<N>` format is allocated by scanning one workspace task tree. That shape is useful as a local search key but fails when tasks need to be referenced across local workspaces, through an explicit registry, hosted Team, or durable design docs without explaining which machine allocated them.

## Decision
Adopt `ORB-00000` as the canonical v2 task ID format: `ORB-` plus a five-digit decimal suffix allocated by an explicit authority. Local-only Orbit uses one machine-local allocator across all workspaces; the ID is unique inside that authority, not across unrelated local registries. V2 task bundles do not preserve old `T...` identifiers as aliases.

## Consequences
- Task IDs become meaningful inside the scope of the configured allocator instead of implicitly workspace-local.
- Local-only Orbit uses one allocator across all local workspaces, so one machine does not mint the same ID for two repositories.
- Two unrelated local registries may both allocate the same bare ID; cross-registry references must carry registry, workspace, hosted tenant, or external-reference context.
- Implementations must stop validating only `T<YYYYMMDD>-<N>` and must add numeric `ORB-\d{5}` validation.
- Existing local tasks need a cutover command, but the result is a clean v2 task store rather than a dual-ID store.
- Cost: task creation now depends on an allocator outside the task directory scan. Sync and hosted modes need shared allocation before a task can be published.