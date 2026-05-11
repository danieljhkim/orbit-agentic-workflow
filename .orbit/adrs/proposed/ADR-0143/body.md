## Context
The current `T<YYYYMMDD>-<N>` format is allocated by scanning one workspace task directories. That shape is useful as a local search key but fails when tasks need to be referenced across workspaces, shared registries, hosted Team, or durable design docs without explaining which machine allocated them.

## Decision
Adopt `ORB-A0001` as the canonical v2 task ID format. The numeric suffix is allocated by an explicit authority: local machine-global for standalone OSS, registry-global for task sync, or hosted allocator for Orbit Team. Migrated tasks preserve prior `T...` identifiers in `legacy_ids`.

## Consequences
- Task IDs become meaningful across the scope of the configured allocator instead of implicitly workspace-local.
- Old commits and docs remain resolvable through `legacy_ids`.
- Implementations must stop validating only `T<YYYYMMDD>-<N>` and must add alias-aware lookup.
- Cost: task creation now depends on an allocator outside the task directory scan. Sync and hosted modes need online allocation before a task can be published.