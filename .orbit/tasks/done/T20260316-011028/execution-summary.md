Removed created_at and updated_at from persisted activity YAML artifacts while preserving runtime Activity timestamps via file-system metadata.

Summary of changes:
- removed timestamp fields from the serialized ActivityFileDocument written to activity YAML files
- changed the file-backed activity store to derive created_at and updated_at from the persisted file on read instead of storing them in YAML
- updated insert/get/update/list paths to return Activity values through the metadata-derived read path
- added focused activity-store tests that verify YAML omits the timestamp keys and runtime reads still expose timestamps

Files touched:
- orbit-store/src/file/activity_store.rs

Validation:
- cargo fmt --all
- cargo test -p orbit-store insert_work_omits_timestamp_fields_from_yaml -- --nocapture
- cargo test -p orbit-store get_activity_returns_runtime_timestamps_without_yaml_fields -- --nocapture
- cargo test -p orbit-store
- cargo test --workspace