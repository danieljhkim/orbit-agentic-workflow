# Snake Case Built-in Naming Migration

**Goal:** Normalize built-in activity and job filenames and identifiers to `snake_case` across bundled assets, tracked repo-local copies, and built-in references.
**Scope:** Built-in activity/job YAML assets, tracked `.orbit` copies, bundled loader keys, built-in job step references, init/refresh behavior, and tests that assert current names.
**Assumptions:** The canonical naming decision is `snake_case` for built-in activities and jobs. User-created custom jobs/activities and historical task bundles are out of scope unless a compatibility mechanism requires touching them.
**Risks:** Renaming built-in IDs can break existing repo-local data, seeded jobs, or job step references if the migration is only partial. The biggest risk is ending up with mixed old/new names that still parse individually but no longer connect at runtime.

## Task 1: Define the canonical rename set

**Files:**
- Modify: `orbit-core/src/command/activity.rs`
- Modify: `orbit-core/src/command/job.rs`
- Review: `orbit-core/assets/activities`
- Review: `orbit-core/assets/jobs`
- Review: `.orbit/activities/active`
- Review: `.orbit/jobs/jobs`

**Steps:**
1. Enumerate every built-in activity and job that still uses kebab-case in either filename or embedded ID.
2. Produce the canonical snake_case mapping for activity IDs, job IDs, and job step `target_id` references.
3. Confirm whether any names are already partially migrated (for example `dispatch_task`) and collapse them onto one final form.

**Done When:**
- there is a single explicit old->new rename map for all affected built-ins
- no built-in rename is left to inference during implementation

## Task 2: Rename bundled and tracked YAML assets

**Files:**
- Modify/Rename: `orbit-core/assets/activities/*.yaml`
- Modify/Rename: `orbit-core/assets/jobs/*.yaml`
- Modify/Rename: `.orbit/activities/active/*.yaml`
- Modify/Rename: `.orbit/jobs/jobs/*.yaml`

**Steps:**
1. Rename bundled activity/job asset filenames to snake_case where needed.
2. Update embedded `activity.id`, `job.job_id`, and any built-in `target_id` references to the canonical snake_case names.
3. Apply the same normalization to the tracked repo-local `.orbit` copies so the checked-in sources of truth stay aligned.
4. Remove any stale duplicate old-name files left behind by the rename.

**Done When:**
- bundled and tracked YAMLs use the same snake_case names
- no built-in YAML still references a kebab-case built-in ID unless intentionally preserved for compatibility

## Task 3: Update loaders, seed behavior, and compatibility handling

**Files:**
- Modify: `orbit-core/src/command/activity.rs`
- Modify: `orbit-core/src/command/job.rs`
- Modify as needed: `orbit-core/src/command/init.rs`
- Modify as needed: store/runtime code that assumes old built-in IDs

**Steps:**
1. Update bundled default asset registration arrays so file keys match the new snake_case IDs.
2. Update any runtime logic, init/refresh paths, or built-in references that still hardcode the old kebab-case names.
3. Decide and implement the compatibility behavior for existing workspaces with persisted old-name built-ins.
4. Prefer a mechanical, testable migration or refresh path over silent mixed-name tolerance.

**Done When:**
- Orbit can initialize and load built-ins without any old/new naming mismatch
- existing checked-in repo-local built-ins are not left stranded by the rename
- the compatibility story for already-seeded old names is explicit and tested

## Task 4: Refresh tests and validation coverage

**Files:**
- Modify: `orbit-core/tests/*`
- Modify: `orbit-cli/tests/*`
- Modify: tests near `parse_rejects_mismatched_file_key_and_id` and bundled job/activity parsing

**Steps:**
1. Update tests that assert current bundled job/activity names.
2. Add or refresh coverage for init/seed behavior so built-ins come out in snake_case consistently.
3. Add a regression check that bundled file keys and embedded IDs still match after the rename.
4. Add coverage for any migration/refresh behavior introduced for old kebab-case built-ins.

**Done When:**
- tests fail if a future built-in reintroduces kebab-case drift or mismatched file key/ID pairs
- migration behavior is covered if implemented

## Final Verification
- `rg -n "approve-task-leader|oversee-orbit-operations|perform-maintenance|resolve-backlogged-task|dispatch-task|execute-task|triage-and-dispatch-task" orbit-core .orbit`
- `cargo test -p orbit-core`
- `cargo test -p orbit-cli`
- a manual diff/review confirming built-in activities and jobs now use one `snake_case` convention end-to-end