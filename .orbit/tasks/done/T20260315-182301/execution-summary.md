Implemented `default_input` support for jobs so baseline values can be declared in job YAML and applied before step execution, with caller-supplied `--input` values taking precedence on key collisions.

Code changes:
- Added `#[serde(default)] default_input: Option<serde_json::Value>` to `Job`
- Threaded `default_input` through job create/update store contracts and file persistence
- Seeded runtime input by merging `job.default_input` first, then overlaying the caller input object
- Added validation that `default_input` must be a JSON object
- Surfaced `default_input` in job CLI JSON and human-readable `job show` output
- Updated default job spec parsing to load and preserve `default_input`

Asset changes:
- Added `default_input: { base: main }` to bundled and live `job_task_pipeline.yaml`
- Removed `base` from the required list in bundled and live `create_branch.yaml`
- Updated the `base` field description to document that `job_task_pipeline` defaults it to `main`

Regression coverage:
- Added runtime tests proving `default_input` satisfies required input when no manual input is provided
- Added runtime tests proving manual input overrides colliding default keys while preserving non-colliding defaults
- Extended the job store round-trip test to verify `default_input` persists through YAML serialization
- Updated the type shape test to lock in the new serialized field

Validation:
- `cargo test -p orbit-core --test job_runtime_behavior -- --nocapture`
- `cargo test -p orbit-store job_write_read_roundtrip_preserves_all_fields -- --nocapture`
- `cargo test -p orbit-types job_shapes_are_stable -- --nocapture`
- `cargo test --workspace`

Operational note:
- I did not run `orbit job run job_task_pipeline` in this repo because that pipeline performs branch creation, implementation, PR, and checkout side effects. The no-`--input` behavior is instead covered by the new runtime tests that validate job-level default input seeding and precedence.