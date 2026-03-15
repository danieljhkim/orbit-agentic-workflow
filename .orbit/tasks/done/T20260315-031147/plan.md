# task.yaml Section-Annotated Serialization

**Goal:** `task.yaml` is written with section comments and fields in the logical grouping order shown above.
**Scope:** `orbit-store/src/file/task_store.rs` only — `serialize_task_doc_yaml` and `TaskFileDocument` field order.
**Assumptions:** `serde_yaml::to_value` correctly escapes all field values (strings, nulls, arrays, timestamps).
**Risks:** Multi-line strings (description, title with special chars) must use YAML block or quoted scalars — must verify `serde_yaml::to_value` output matches expected YAML for each type.

## Task 1: Reorder TaskFileDocument struct fields

**Files:**
- Modify: `orbit-store/src/file/task_store.rs` — `TaskFileDocument` field order

**Steps:**
1. Reorder fields in `TaskFileDocument` to match the section order (identity → content → context → ownership → proposal → implementation → review → execution refs → timestamps → audit trail).
2. Run: `cargo test -p orbit-store` (no behavior change expected).

**Done When:** Fields are in the correct order; all tests pass.

## Task 2: Replace serialize_task_doc_yaml with manual formatter

**Files:**
- Modify: `orbit-store/src/file/task_store.rs` — `serialize_task_doc_yaml`

**Steps:**
1. Write a helper `yaml_field(key: &str, value: &impl Serialize) -> String` that returns `"key: <serde_yaml scalar>\n"` using `serde_yaml::to_value`.
2. Write a helper `yaml_section(name: &str) -> String` that returns `"\n# ---- name ----\n"`.
3. Replace the `serde_yaml::to_string(doc)` call with a string builder that emits each section header then each field.
4. Ensure multi-line strings (description) use the `|-` block scalar (verify `serde_yaml::to_value` output or use `serde_yaml::Value::String` directly).
5. Run: `cargo test -p orbit-store`.

**Done When:** `cargo test -p orbit-store` passes. The generated YAML contains section comments in the correct positions.

## Task 3: Add a targeted serialization test

**Files:**
- Modify: `orbit-store/src/file/task_store.rs` — `#[cfg(test)]` block

**Steps:**
1. Add test `task_yaml_contains_section_comments_in_order` that creates a task, reads the raw YAML, and asserts:
   - `# ---- identity ----` appears before `id:`
   - `# ---- content ----` appears before `title:`
   - `# ---- audit trail ----` appears before `history:`
2. Run: `cargo test -p orbit-store task_yaml_contains_section_comments_in_order`.

**Done When:** Test passes.

## Final Verification
```
cargo test -p orbit-store
orbit task add --title "fmt-test" --description "multi\nline" --plan "p" --proposed-by system
# Inspect the generated task.yaml manually to confirm section comments are present
```