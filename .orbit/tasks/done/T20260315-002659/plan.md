# Activity spec_type Variants Implementation Plan

**Goal:** Replace the unvalidated `spec_type` string with an execution-branched system supporting `agent_invoke`, `cli_command`, and `api`.
**Scope:** orbit-types, orbit-store, orbit-core, orbit-cli, asset YAMLs, tests. No DB migration needed (file store only).
**Assumptions:** T20260315-000227 is complete before starting. reqwest is added as a workspace dep.
**Risks:** The `build_stdin_envelope_payload` function and the `execute_single_attempt` function in job.rs need careful surgical edits. Test coverage for cli_command and api execution paths requires careful mocking.

---

## Task 1: Migrate `Activity` struct to `spec_config: Value`

**Files:**
- Modify: `orbit-types/src/activity.rs`

**Steps:**
1. Remove fields: `instruction: String`, `skill_refs: Vec<String>`, `tools: Vec<String>`
2. Add field: `spec_config: serde_json::Value` (default to empty object)
3. Run: `cargo check -p orbit-types`

**After change, Activity struct is:**
```rust
pub struct Activity {
    pub id: OrbitId,
    pub spec_type: String,
    pub description: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub spec_config: Value,         // type-specific config (flat YAML fields assembled here)
    pub identity_id: Option<String>,
    pub created_by: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Done When:** `cargo check -p orbit-types` passes.

---

## Task 2: Update `orbit-store` contracts and file store

**Files:**
- Modify: `orbit-store/src/backend/contracts.rs`
- Modify: `orbit-store/src/file/activity_store.rs`
- Modify: `orbit-store/src/backend/file_backends.rs`

**Steps:**

### 2a. contracts.rs
- `ActivityCreateParams`: remove `instruction`, `skill_refs`, `tools`; add `spec_config: Value`
- `ActivityUpdateParams`: same removals/addition

### 2b. activity_store.rs
- `FileWorkInsert`: remove `instruction`/`skill_refs`/`tools`; add `spec_config: Value`
- `ActivitySpecDocument`: remove those fields; add `#[serde(flatten)] spec_config: serde_json::Map<String, Value>`
  - This serializes spec_config fields flat into YAML (e.g., `instruction: ...`, `skill_refs: [...]`) and deserializes unknown fields back into spec_config
- `insert_work`: use `params.spec_config.as_object().cloned().unwrap_or_default()` to populate `ActivitySpecDocument.spec_config`
- `doc_to_work` (the function that converts `ActivityFileDocument` → `Activity`): map `doc.activity.spec_config` (Map) into `Value::Object` for the `Activity.spec_config` field

### 2c. file_backends.rs
- Remove `instruction`/`skill_refs`/`tools` from both `ActivityCreateParams` → `FileWorkInsert` bridge and `ActivityUpdateParams` bridge
- Add `spec_config`

**Done When:** `cargo check -p orbit-store` passes.

---

## Task 3: Update `orbit-core/src/command/activity.rs`

**Files:**
- Modify: `orbit-core/src/command/activity.rs`

**Steps:**

### 3a. `ActivityAddParams`
Remove `instruction`, `skill_refs`, `tools`; add `spec_config: Value`

### 3b. `ActivityUpdateParams`
Same — remove the three fields, add `spec_config: Option<Value>`

### 3c. `ActivityFileSpec` (internal YAML parsing struct)
Replace flat type-specific fields with `#[serde(flatten)] spec_config: serde_json::Map<String, Value>`:
```rust
#[derive(Debug, Clone, Deserialize)]
struct ActivityFileSpec {
    id: String,
    spec_type: String,
    description: String,
    #[serde(default)]
    input_schema_json: Value,
    #[serde(default)]
    output_schema_json: Value,
    #[serde(flatten)]
    spec_config: serde_json::Map<String, Value>,
}
```

### 3d. `add_activity` method
Update to pass `spec_config: Value::Object(spec.spec_config)` instead of flat fields. Keep skill_refs resolution — extract `skill_refs` from spec_config for validation: `spec_config.get("skill_refs").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()`

### 3e. `validate_activity_params`
Add: reject `spec_type` values outside `["agent_invoke", "cli_command", "api"]` with a clear error message listing valid values.

**Done When:** `cargo check -p orbit-core` passes.

---

## Task 4: Add template engine — `orbit-core/src/template.rs`

**Files:**
- Create: `orbit-core/src/template.rs`
- Modify: `orbit-core/src/lib.rs` (add `pub mod template;`)

**Template engine spec:**
- Input: a string containing `{{namespace.key}}` tokens and a `TemplateContext`
- Output: rendered string or `OrbitError::InvalidInput` if a key is missing

```rust
pub struct TemplateContext {
    pub input: serde_json::Value,           // input_schema_json values at runtime
    pub env: std::collections::HashMap<String, String>,
    pub workspace_path: Option<String>,
}

pub fn render(template: &str, ctx: &TemplateContext) -> Result<String, OrbitError> { ... }
```

**Parsing logic (hand-rolled, no extra deps):**
1. Scan for `{{` ... `}}` patterns
2. Split token on `.` to get `(namespace, key)`
3. Dispatch by namespace:
   - `input` → look up in `ctx.input` as JSON object field, serialize to string
   - `env` → `ctx.env.get(key)`
   - `workspace_path` (no key, just the token itself) → `ctx.workspace_path.clone()`
   - `secrets` → return `OrbitError::InvalidInput("secrets namespace not yet supported")`
   - unknown namespace → return `OrbitError::InvalidInput("unknown template namespace: {ns}")`
4. Replace each match with the resolved value

**Tests (in same file, `#[cfg(test)]`):**
- `input.field` substitution
- `env.VAR` substitution
- `workspace_path` substitution
- Missing key → error
- `secrets.*` → error
- No tokens → returns unchanged string

**Done When:** `cargo test -p orbit-core template` passes.

---

## Task 5: Add CLI command executor — `orbit-core/src/executor/`

**Files:**
- Create: `orbit-core/src/executor/mod.rs`
- Create: `orbit-core/src/executor/cli_command.rs`
- Modify: `orbit-core/src/lib.rs` (add `pub mod executor;`)

**`CliCommandSpec` struct** (deserializes from `activity.spec_config`):
```rust
#[derive(Debug, Deserialize)]
pub struct CliCommandSpec {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub timeout_seconds: Option<u64>,
    #[serde(default = "default_exit_codes")]
    pub expected_exit_codes: Vec<i32>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}
fn default_exit_codes() -> Vec<i32> { vec![0] }
```

**Execution logic:**
1. Deserialize `spec_config` into `CliCommandSpec`
2. Apply template substitution to: `command`, each `args` element, each `env` value, `working_dir`
3. Create a temp file path for `ORBIT_OUTPUT_FILE`
4. Build `std::process::Command`:
   - Inherit env, then overlay `spec.env` (already substituted)
   - Set `ORBIT_OUTPUT_FILE` to temp path
   - Set `current_dir` if `working_dir` is set
5. Spawn and wait with timeout:
   - Use a background thread + `child.wait()` + channel with `recv_timeout` for timeout handling
   - On timeout: kill child, return `OrbitError::Execution("cli_command timed out")`
6. Check exit code against `expected_exit_codes`; if not in list, return error with exit code
7. Read `ORBIT_OUTPUT_FILE` if it exists and parse as JSON; otherwise return `json!({"exit_code": code})`
8. Return `serde_json::Value` as the step result

**Done When:** Unit test with `command: echo`, `args: [hello]` produces `{"exit_code": 0}`.

---

## Task 6: Add HTTP API executor — `orbit-core/src/executor/api.rs`

**Files:**
- Create: `orbit-core/src/executor/api.rs`
- Modify: `orbit-core/Cargo.toml` — add `reqwest` with `blocking` and `json` features

Check workspace `Cargo.toml` first — if `reqwest` is already a workspace dep, use `reqwest.workspace = true`. Otherwise add: `reqwest = { version = "0.12", features = ["blocking", "json"] }`

**`ApiSpec` struct:**
```rust
#[derive(Debug, Deserialize)]
pub struct ApiSpec {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    pub timeout_seconds: Option<u64>,
    #[serde(default = "default_status_codes")]
    pub expected_status_codes: Vec<u16>,
}
fn default_status_codes() -> Vec<u16> { vec![200] }
```

**Execution logic:**
1. Deserialize `spec_config` into `ApiSpec`
2. Apply template substitution to: `url`, all `headers` values, `body`
3. Build `reqwest::blocking::Client` with timeout from `timeout_seconds` (default 30s)
4. Build request: method, url, headers
5. If `body` is set, set request body (content-type from headers, or assume JSON)
6. Execute; check `response.status().as_u16()` is in `expected_status_codes`; else error
7. Parse response body as JSON; if not valid JSON, wrap as `json!({"body": raw_string})`
8. Return the parsed `Value` as the step result

**Done When:** `cargo check -p orbit-core` passes. (Full integration test requires network access — skip in unit tests.)

---

## Task 7: Update execution dispatch in `orbit-core/src/command/job.rs`

**Files:**
- Modify: `orbit-core/src/command/job.rs`

**Steps:**

### 7a. `build_stdin_envelope_payload`
The `activity` block in the envelope JSON currently includes flat `instruction`, `skill_refs`, `tools`. Change to merge `spec_config` into the envelope:
```rust
let mut activity_json = serde_json::json!({
    "id": execution.activity.id,
    "type": execution.activity.spec_type,
    "description": execution.activity.description,
    "input_schema_json": execution.activity.input_schema_json,
    "output_schema_json": execution.activity.output_schema_json,
    "identity_id": execution.activity.identity_id,
    "created_by": execution.activity.created_by,
});
// Merge spec_config fields into the activity envelope
if let Value::Object(config) = &execution.activity.spec_config {
    if let Value::Object(map) = &mut activity_json {
        for (k, v) in config { map.insert(k.clone(), v.clone()); }
    }
}
```

Also update the `skill_refs` resolution used in `build_stdin_envelope_payload` — extract `skill_refs` from `spec_config`:
```rust
let skill_refs: Vec<String> = execution.activity.spec_config
    .get("skill_refs")
    .and_then(|v| serde_json::from_value(v.clone()).ok())
    .unwrap_or_default();
let skills = self.resolve_activity_skill_refs(&skill_refs)?;
```

### 7b. `execute_single_attempt` — add execution dispatch
Locate the code that calls `agent.invoke(...)`. Wrap it in a match:

```rust
match execution.activity.spec_type.as_str() {
    "agent_invoke" | "" => {
        // existing LLM agent path
    }
    "cli_command" => {
        let ctx = build_template_context(&execution);
        let spec: CliCommandSpec = serde_json::from_value(execution.activity.spec_config.clone())
            .map_err(|e| OrbitError::InvalidInput(format!("invalid cli_command spec: {e}")))?;
        let result = execute_cli_command(&spec, &ctx)?;
        // finalize job run as succeeded with result as agent_response_json
    }
    "api" => {
        let ctx = build_template_context(&execution);
        let spec: ApiSpec = serde_json::from_value(execution.activity.spec_config.clone())
            .map_err(|e| OrbitError::InvalidInput(format!("invalid api spec: {e}")))?;
        let result = execute_api(&spec, &ctx)?;
        // finalize job run as succeeded
    }
    other => return Err(OrbitError::InvalidInput(format!("unknown spec_type: {other}"))),
}
```

### 7c. Add `build_template_context` helper
```rust
fn build_template_context(execution: &ExecutionContext) -> TemplateContext {
    TemplateContext {
        input: execution.input.clone(),
        env: std::env::vars().collect(),
        workspace_path: execution.activity.workspace_path.clone(),  // if present on Activity; else None
    }
}
```

Note: also skip `validate_skill_output_schema` for `cli_command` and `api` (no skills involved).

**Done When:** `cargo check -p orbit-core` passes.

---

## Task 8: Update `orbit-cli/src/command/activity.rs`

**Files:**
- Modify: `orbit-cli/src/command/activity.rs`

**Steps:**
1. `ActivityAddArgs`: remove `--instruction`, `--skill-refs`, `--tools` flags; add `--spec-config <JSON>` (`Option<String>`, parsed as `serde_json::Value`)
2. `ActivityUpdateArgs`: same — remove old flags, add `--spec-config <JSON>`
3. Conversion from `ActivityAddArgs` → `ActivityAddParams`: parse `--spec-config` as JSON, default to `{}`
4. `activity_to_json`: remove `instruction`/`skill_refs`/`tools` fields; add `spec_config` field
5. Run: `cargo check -p orbit-cli`

**Done When:** `cargo check -p orbit-cli` passes. `orbit activity add --help` shows `--spec-config` flag.

---

## Task 9: Rewrite 5 asset YAML files to use `agent_invoke`

**Files:**
- Modify: `orbit-core/assets/activities/approve-task-leader.yaml`
- Modify: `orbit-core/assets/activities/oversee-orbit-operations.yaml`
- Modify: `orbit-core/assets/activities/perform-maintenance.yaml`
- Modify: `orbit-core/assets/activities/resolve-backlogged-task.yaml`
- Modify: `orbit-core/assets/activities/triage-and-dispatch-task.yaml`

**For each file:**
- Change `spec_type` to `agent_invoke` (from whatever value it currently has: `task`, `task_approval`, `operation_management`, etc.)
- Keep all other flat fields as-is (instruction, skill_refs, etc. remain flat — the new serde(flatten) logic will assemble them into spec_config)
- Verify the envelope structure matches the new schema from T20260315-000227 (snake_case, schema_version at top, activity: block)

**Done When:** All 5 files have `spec_type: agent_invoke`. `cargo check` passes.

---

## Task 10: Delete stale .orbit activity artifacts and update tests

**Files:**
- Delete: `.orbit/activities/active/*.yaml` (stale, old format — will be re-seeded by `orbit init`)
- Modify: `orbit-cli/tests/activity_commands.rs`

**Test updates:**
1. Remove any test that references `--instruction`, `--skill-refs`, `--tools`, `--artifact-path-template`, `--assigned-to` CLI flags
2. Update `activity_to_json` assertions to use `spec_config` instead of flat fields
3. Add a test: `activity_add_with_cli_command_spec` — creates an activity with `spec_type: cli_command` and `--spec-config '{"command": "echo", "args": ["hello"]}'`, verifies it persists and `activity_to_json` returns the spec_config correctly

**Stale artifact cleanup:**
```bash
rm -f .orbit/activities/active/*.yaml
orbit init  # re-seeds from bundled assets
```

Verify with: `orbit activity list` — should show 5 activities all with `spec_type: agent_invoke`

---

## Final Verification

```bash
# Full compile
cargo build --workspace

# All tests
cargo test --workspace

# Confirm spec_config stored correctly
orbit activity add --spec-type agent_invoke --description 'test'   --spec-config '{"instruction": "do something", "skill_refs": []}'   --workspace . --id test-agent-invoke
orbit activity show test-agent-invoke --json | jq .spec_config

# Confirm CLI command activity can be added
orbit activity add --spec-type cli_command --description 'echo test'   --spec-config '{"command": "echo", "args": ["hello"]}'   --workspace . --id test-cli-cmd
orbit activity show test-cli-cmd --json | jq .spec_config

# Confirm no references to old flat fields remain in activity JSON output
orbit activity list --json | jq '.[].instruction'  # should be null for all

# Confirm old spec_type values are gone
grep -r 'specType\|spec_type.*task_approval\|spec_type.*task_dispatch' orbit-core/assets/
# should return nothing
```