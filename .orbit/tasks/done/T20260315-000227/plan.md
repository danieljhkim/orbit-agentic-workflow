# Activity Schema Refactor Plan

**Goal:** Migrate the activity artifact format to snake_case with an envelope wrapper, removing assigned_to and artifact_path_template, adding tools field.
**Scope:** orbit-types, orbit-store, orbit-core, orbit-cli, asset YAML files, all tests.
**Assumptions:** No migration of existing on-disk files needed — they will be deleted and re-seeded.
**Risks:** Wide surface area; compile errors will cascade. Work crate-by-crate bottom-up.

---

## Task 1: Update `orbit-types/src/activity.rs`

**Files:**
- Modify: `orbit-types/src/activity.rs`

**Steps:**
1. Write failing test in `orbit-types/src/lib.rs` that constructs an `Activity` with a `tools` field and no `assigned_to` / `artifact_path_template`.
2. Run: `cargo test -p orbit-types 2>&1 | head -40`
3. In `orbit-types/src/activity.rs`:
   - Remove field: `pub assigned_to: Option<String>`
   - Remove field: `pub artifact_path_template: Option<String>`
   - Add field: `#[serde(default)] pub tools: Vec<String>`
4. Re-run: `cargo test -p orbit-types 2>&1 | head -40`

**Done When:**
- `orbit-types` compiles and its tests pass.

---

## Task 2: Update `orbit-store/src/backend/contracts.rs`

**Files:**
- Modify: `orbit-store/src/backend/contracts.rs`

**Steps:**
1. In `ActivityCreateParams`:
   - Remove: `artifact_path_template: Option<String>`
   - Remove: `assigned_to: Option<String>`
   - Add: `tools: Vec<String>`
2. In `ActivityUpdateParams`:
   - Remove: `artifact_path_template: Option<Option<String>>`
   - Remove: `assigned_to: Option<Option<String>>`
   - Add: `tools: Option<Vec<String>>`
3. Run: `cargo build -p orbit-store 2>&1 | head -60`

**Done When:**
- `orbit-store` compiles (will have downstream errors in file_backends.rs — fix those next).

---

## Task 3: Update `orbit-store/src/file/activity_store.rs`

**Files:**
- Modify: `orbit-store/src/file/activity_store.rs`

**Steps:**
1. Restructure `ActivityFileDocument` to match the new on-disk format. Remove `#[serde(rename_all = "camelCase")]`. Split into envelope + nested spec:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivitySpecDocument {
    id: String,
    spec_type: String,
    description: String,
    #[serde(default)]
    instruction: String,
    input_schema_json: serde_json::Value,
    output_schema_json: serde_json::Value,
    #[serde(default)]
    skill_refs: Vec<String>,
    #[serde(default)]
    tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivityFileDocument {
    schema_version: u8,
    #[serde(default)]
    created_by: Option<String>,
    #[serde(default)]
    identity_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    activity: ActivitySpecDocument,
}
```

2. Update `FileWorkInsert`:
   - Remove: `artifact_path_template`, `assigned_to`
   - Add: `tools: Vec<String>`

3. Update `insert_work` to build the new nested document structure.

4. Update `update_activity` signature and body:
   - Remove `artifact_path_template` and `assigned_to` params
   - Add `tools: Option<Vec<String>>`
   - Apply updates to `doc.activity.*` fields (id, spec_type, description, etc. live inside the nested struct)
   - `identity_id`, `created_by`, `is_active` stay at envelope level

5. Update `doc_to_work` to map from nested struct back to flat `Activity`.

6. Run: `cargo build -p orbit-store 2>&1 | head -60`

**Done When:**
- `orbit-store` compiles and tests pass: `cargo test -p orbit-store 2>&1 | tail -20`

---

## Task 4: Update `orbit-store/src/backend/file_backends.rs`

**Files:**
- Modify: `orbit-store/src/backend/file_backends.rs`

**Steps:**
1. In `ActivityStoreBackend for ActivityFileStore`:
   - Remove `artifact_path_template` from `FileWorkInsert` construction.
   - Remove `assigned_to` from `FileWorkInsert` construction.
   - Add `tools: params.tools` to `FileWorkInsert`.
2. In the `update_activity` call:
   - Remove `params.artifact_path_template` arg.
   - Remove `params.assigned_to` arg.
   - Add `params.tools` arg.
3. Run: `cargo build -p orbit-store 2>&1 | head -60`

**Done When:**
- `orbit-store` compiles and all its tests pass.

---

## Task 5: Update `orbit-core/src/command/activity.rs`

**Files:**
- Modify: `orbit-core/src/command/activity.rs`

**Steps:**
1. Introduce a new intermediate struct for parsing the new YAML envelope format used by asset files. This replaces the current direct parse into `ActivityAddParams`:

```rust
#[derive(Debug, Clone, Deserialize)]
struct ActivityFileEnvelope {
    schema_version: u8,
    #[serde(default)]
    created_by: Option<String>,
    #[serde(default)]
    identity_id: Option<String>,
    activity: ActivityFileSpec,
}

#[derive(Debug, Clone, Deserialize)]
struct ActivityFileSpec {
    id: String,
    spec_type: String,
    description: String,
    #[serde(default)]
    instruction: String,
    #[serde(default)]
    input_schema_json: serde_json::Value,
    #[serde(default)]
    output_schema_json: serde_json::Value,
    #[serde(default)]
    skill_refs: Vec<String>,
    #[serde(default)]
    tools: Vec<String>,
}
```

2. Update `load_default_activity_specs` to parse `ActivityFileEnvelope` instead of `ActivityAddParams`, then convert:
   - `spec.activity.id` → `params.id`
   - `spec.identity_id` → `params.identity_id`
   - `spec.created_by` → `params.created_by`
   - `spec.activity.tools` → `params.tools`
   - etc.

3. Update `ActivityAddParams`:
   - Remove: `artifact_path_template`, `assigned_to`
   - Add: `tools: Vec<String>` with `#[serde(default)]`

4. Update `ActivityUpdateParams`:
   - Remove: `artifact_path_template`, `assigned_to`
   - Add: `tools: Option<Vec<String>>`

5. Update `add_activity`:
   - Remove the identity-resolution block that derived `assigned_to` from `identity_id`. The `identity_id` is stored directly; no derivation needed.
   - Remove `assigned_to` from `StoreWorkCreateParams`.
   - Add `tools: params.tools` to `StoreWorkCreateParams`.

6. Update `update_activity`:
   - Remove `assigned_to` from `StoreActivityUpdateParams`.
   - Add `tools: params.tools` to `StoreActivityUpdateParams`.

7. Run: `cargo build -p orbit-core 2>&1 | head -60`

**Done When:**
- `orbit-core` compiles.

---

## Task 6: Remove `artifact_path_template` from agent execution context in `orbit-core/src/command/job.rs`

**Files:**
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Search for `artifact_path_template` in `job.rs` (around line 761). Remove it from the JSON object passed to the agent prompt.
2. Run: `cargo build -p orbit-core 2>&1 | head -40`

**Done When:**
- `orbit-core` compiles without errors.

---

## Task 7: Update `orbit-cli/src/command/activity.rs`

**Files:**
- Modify: `orbit-cli/src/command/activity.rs`

**Steps:**
1. In `ActivityAddArgs`:
   - Remove: `artifact_path_template: Option<String>` field and its `#[arg(long)]`
   - Remove: `assigned_to: Option<String>` field and its `#[arg(long)]`
   - (Keep `created_by` if it exists; it maps to the envelope-level `created_by`)

2. In `ActivityAddArgs::execute`:
   - Remove `artifact_path_template: self.artifact_path_template` from `ActivityAddParams`
   - Remove `assigned_to: self.assigned_to` from `ActivityAddParams`

3. In `ActivityUpdateArgs`:
   - Remove: `artifact_path_template: Option<String>` and `#[arg(long)]`
   - Remove: `clear_artifact_path_template: bool` and its `#[arg(long, conflicts_with = ...)]`
   - Remove: `assigned_to: Option<String>` and `#[arg(long)]`
   - Remove: `clear_assigned_to: bool` and its `#[arg(long, conflicts_with = ...)]`

4. In `ActivityUpdateArgs::execute`:
   - Remove the `artifact_path_template` derivation block
   - Remove the `assigned_to` derivation block
   - Remove them from `ActivityUpdateParams`

5. In `activity_to_json`:
   - Remove: `"artifact_path_template": spec.artifact_path_template`
   - Remove: `"assigned_to": spec.assigned_to`
   - Add: `"tools": spec.tools`

6. In `ActivityShowArgs::execute` (text output path):
   - Remove: `println!("Artifact Template: ...")`
   - Remove: `if let Some(ref assigned_to) = spec.assigned_to { ... }`
   - Add: `println!("Tools: {}", spec.tools.join(","))`

7. Run: `cargo build -p orbit-cli 2>&1 | head -60`

**Done When:**
- `orbit-cli` compiles.

---

## Task 8: Rewrite asset YAML files to new format

**Files:**
- Modify: `orbit-core/assets/activities/approve-task-leader.yaml`
- Modify: `orbit-core/assets/activities/oversee-orbit-operations.yaml`
- Modify: `orbit-core/assets/activities/perform-maintenance.yaml`
- Modify: `orbit-core/assets/activities/resolve-backlogged-task.yaml`
- Modify: `orbit-core/assets/activities/triage-and-dispatch-task.yaml`

**Steps:**
1. For each YAML file, rewrite it to the new format. Example transformation for `approve-task-leader.yaml`:

Before:
```yaml
id: approve-task-leader
specType: task_approval
description: ...
skillRefs: [...]
identityId: prii
assignedTo: prii
createdBy: system
```

After:
```yaml
schema_version: 1
created_by: system
identity_id: prii
activity:
    id: approve-task-leader
    spec_type: task_approval
    description: ...
    instruction: ...
    input_schema_json: {}
    output_schema_json: {}
    skill_refs: [...]
    tools: []
```

Apply the same pattern to all 5 files. Note: `input_schema_json` and `output_schema_json` must be valid YAML objects. Preserve the existing `instruction`, `inputSchemaJson`/`outputSchemaJson` content, converting keys to snake_case.

Note that `additional_properties` in JSON Schema is also snake_case in YAML: use `additional_properties: false` (serde_json will accept this if the struct uses the default field naming).

2. Run the unit test that validates bundled activity specs parse successfully:
   `cargo test -p orbit-core bundled_default_activity_specs_parse_successfully 2>&1`

**Done When:**
- All 5 asset files parse without error in the test above.

---

## Task 9: Update all tests

**Files:**
- Modify: `orbit-types/src/lib.rs`
- Modify: `orbit-core/src/command/activity.rs` (inline tests)
- Modify: `orbit-core/tests/job_runtime_behavior.rs`
- Modify: `orbit-cli/tests/activity_commands.rs`

**Steps:**

### orbit-types/src/lib.rs
- In the `Activity` shape test: remove `assigned_to`, `artifact_path_template`; add `tools: vec![]`.

### orbit-core/src/command/activity.rs (inline tests)
- `parse_rejects_duplicate_ids`, `parse_rejects_empty_ids`, `parse_rejects_mismatched_file_key_and_id`, `parse_replaces_orbit_root_token_when_provided`, `bundled_default_activity_specs_parse_successfully`:
  - Update inline YAML fixture strings to new envelope format.
  - The `parse_replaces_orbit_root_token_when_provided` test uses `artifactPathTemplate` — update to use a different field or remove that assertion since `artifact_path_template` is removed. Instead test token substitution via a `skill_refs` value or `description` that contains `{{ORBIT_ROOT}}`.

### orbit-core/tests/job_runtime_behavior.rs
- All `ActivityAddParams` usages: remove `artifact_path_template: None`, remove `assigned_to: None`, add `tools: vec![]`.
- There are ~6 occurrences. Search with: `grep -n "artifact_path_template\|assigned_to" orbit-core/tests/job_runtime_behavior.rs`

### orbit-cli/tests/activity_commands.rs
- Remove the `activity_update_clear_artifact_path_template` test entirely (the feature no longer exists).
- Any test that uses `--artifact-path-template` or `--assigned-to` CLI flags: remove those args.
- Any test asserting `show["artifact_path_template"]` or `show["assigned_to"]`: remove those assertions.
- If any test asserts absence of a field in signal-tier JSON (the `--ops` test), verify it still makes sense.

---

## Task 10: Delete existing orbit activity artifacts and verify full suite

**Steps:**
1. Delete stale activity files (they will be re-seeded by `orbit init`):
   ```
   rm -f .orbit/activities/active/*.yaml
   rm -f .orbit/activities/inactive/*.yaml
   ```
2. Run the full test suite:
   ```
   cargo test --workspace 2>&1 | tail -40
   ```
3. Fix any remaining compilation errors or test failures.

**Done When:**
- `cargo test --workspace` exits 0.
- No references to `artifact_path_template` or `assigned_to` remain in activity-related code paths (check with `grep -r "artifact_path_template\|assigned_to" orbit-types/src orbit-store/src orbit-core/src orbit-cli/src`).

---

## Final Verification

```bash
cargo test --workspace 2>&1 | tail -40
grep -r "artifact_path_template" orbit-types/src orbit-store/src orbit-core/src orbit-cli/src
grep -r "assigned_to" orbit-types/src/activity.rs orbit-store/src/file/activity_store.rs orbit-core/src/command/activity.rs orbit-cli/src/command/activity.rs
```

Expected: test suite green, no remaining hits for removed fields in activity-specific files.