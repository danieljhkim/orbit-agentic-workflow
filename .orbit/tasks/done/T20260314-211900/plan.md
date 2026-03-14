# Seed Default Jobs from YAML Plan

**Goal:** Replace hardcoded job seed constants with `include_str!`-embedded asset YAML parsing.
**Scope:** `orbit-core/src/command/job.rs` and its unit tests only.
**Assumptions:** Asset YAML files in `orbit-core/assets/jobs/` are already accurate and complete.
**Risks:** YAML schema mismatch between asset files and the new deserializer struct; mitigated by adding a parse test that covers all bundled files.

## Task 1: Add failing test

**Files:**
- Modify: `orbit-core/src/command/job.rs` (add test module or extend existing)

**Steps:**
1. Add a test `bundled_default_job_specs_parse_successfully` that calls a new `load_default_job_specs` function with all asset files.
2. Run `cargo test -p orbit-core bundled_default_job_specs` — expect compile error / missing function.

**Done When:**
- Test file compiles and the test fails with "function not found" or similar.

## Task 2: Define deserialization types and `load_default_job_specs`

**Files:**
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Define:
   ```rust
   #[derive(Debug, Clone, Deserialize)]
   struct DefaultJobFileSpec {
       #[serde(rename = "schemaVersion")]
       schema_version: u32,
       job: DefaultJobEntry,
   }

   #[derive(Debug, Clone, Deserialize)]
   struct DefaultJobEntry {
       job_id: String,
       target_type: String,
       target_id: String,
       agent_cli: String,
       timeout_seconds: u64,
       state: String,
       #[serde(default)]
       env_extra: Vec<String>,
   }
   ```
2. Add `DEFAULT_JOB_FILES: &[(&str, &str)]` const with `include_str!` for each asset file.
3. Implement `load_default_job_specs(raw_specs: &[(&str, &str)]) -> Result<Vec<...>, OrbitError>` mirroring `load_default_activity_specs`.
4. Run test again — should pass.

**Done When:**
- `cargo test -p orbit-core bundled_default_job_specs` passes.

## Task 3: Refactor `seed_default_jobs`

**Files:**
- Modify: `orbit-core/src/command/job.rs`

**Steps:**
1. Replace `DEFAULT_NAMED_JOBS` const with the new `DEFAULT_JOB_FILES` const.
2. Rewrite `seed_default_jobs` to call `load_default_job_specs`, then iterate and call `add_job` with parsed data (mapping `state` → `JobScheduleState`, `target_type` → `JobTargetType`).
3. Remove old `DEFAULT_NAMED_JOBS`.
4. Run full test suite: `cargo test -p orbit-core && cargo test -p orbit-cli`.

**Done When:**
- All existing tests pass.
- `seed_default_jobs` no longer references `DEFAULT_NAMED_JOBS`.
- The seeded jobs are identical to what was seeded before (same job_id, target_type, target_id, agent_cli, timeout_seconds, env_extra).

## Final Verification

```bash
cargo test -p orbit-core
cargo test -p orbit-cli
cargo build
orbit init --force
orbit job list
```

Expected: all 5 default jobs present in `orbit job list`.