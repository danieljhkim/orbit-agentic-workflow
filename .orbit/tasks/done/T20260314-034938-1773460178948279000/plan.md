# Job Retry Removal Implementation Plan

**Goal:** Remove retry-related job configuration and keep job execution/persistence intentionally simple.
**Scope:** Delete retry knobs from job definitions, APIs, storage, and runtime while preserving existing non-retry job behavior.
**Assumptions:** Single-attempt execution is the desired permanent behavior for scheduled and manual jobs.
**Risks:** Persistence changes may require careful migration updates to avoid breaking existing SQLite-backed workspaces.

## Task 1: Remove retry fields from external job surfaces

**Files:**
- Modify: `.orbit/jobs/jobs/*.yaml`
- Modify: `orbit-cli/src/command/job.rs`
- Modify: `orbit-cli/tests/job_commands.rs`

**Steps:**
1. Remove retry properties from committed job YAML definitions and any related fixture/setup data.
2. Remove retry flags from `orbit job add` and retry fields from human/json output where they are exposed.
3. Update CLI coverage so job creation/show/list assertions reflect the simplified schema.

**Done When:**
- Job definitions no longer declare retry properties.
- CLI users can add/show jobs without retry-specific inputs or outputs.

## Task 2: Simplify core job model and runtime

**Files:**
- Modify: `orbit-types/src/job.rs`
- Modify: `orbit-core/src/command/job.rs`
- Modify: `orbit-core/src/lib.rs`
- Modify: `orbit-core/tests/job_runtime_behavior.rs`

**Steps:**
1. Remove retry fields and backoff types from shared job models and command-layer add params.
2. Refactor runtime execution so a triggered job performs a single run result path instead of retry bookkeeping/backoff scheduling.
3. Update or replace runtime tests so they validate single-attempt execution semantics directly.

**Done When:**
- Shared job structs no longer carry retry properties.
- Runtime logic no longer computes retry attempts/backoff delays.

## Task 3: Remove retry data from persistence and contracts

**Files:**
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/backend/file_backends.rs`
- Modify: `orbit-store/src/backend/sqlite_backends.rs`
- Modify: `orbit-store/src/file/job_store.rs`
- Modify: `orbit-store/src/sqlite/job_store.rs`
- Modify: `orbit-store/src/sqlite/migration.rs`
- Modify: `orbit-store/migrations/0001_init.sql`

**Steps:**
1. Remove retry fields from backend create/load contracts and file-backed serialization.
2. Remove retry columns from SQLite queries/schema definitions and add any required forward migration so existing databases remain readable.
3. Adjust persistence tests/fixtures to match the reduced schema.

**Done When:**
- New persisted jobs do not store retry properties.
- SQLite/file backends operate with the simplified schema.

## Final Verification
- `cargo test -p orbit-cli job_commands -- --nocapture`
- `cargo test -p orbit-core job_runtime_behavior -- --nocapture`
- `cargo test -p orbit-store`
- `cargo test --workspace`