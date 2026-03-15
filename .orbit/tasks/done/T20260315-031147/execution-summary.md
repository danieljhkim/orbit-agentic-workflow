Reworked `task.yaml` serialization to emit section comments and a human-oriented field order while preserving YAML escaping through per-field `serde_yaml` serialization.

Summary of changes:
- reordered `TaskFileDocument` fields to match the intended section grouping for task metadata
- replaced the old whole-document `serde_yaml::to_string` path with a manual string builder in `serialize_task_doc_yaml`
- added small helpers that serialize one field at a time via `serde_yaml::to_value`/`serde_yaml::to_string`, then insert section headers between logical groups
- added a focused regression test that verifies section comments appear before the expected field groups in the generated YAML

Strategic decisions:
- kept `serde_yaml` responsible for each field's scalar/collection rendering instead of hand-formatting YAML values | Rationale: preserves quoting, nulls, arrays, timestamps, and multiline string behavior with less risk | Trade-offs: the section layout is custom, but field rendering still depends on serde_yaml output conventions

Validation:
- cargo test -p orbit-store task_yaml_contains_section_comments_in_order
- cargo test -p orbit-store