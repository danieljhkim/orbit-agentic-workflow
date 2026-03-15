Added `--json` support across the remaining mutating Orbit CLI commands so agents and scripts can consume structured output consistently.

Summary of changes:
- added `--json` output handling to `task add`, `task update`, `task approve`, `task reject`, `task archive`, `task unarchive`, and `task delete`
- added `--json` delete responses for `activity delete` and `job delete`
- reused existing task/activity/job JSON helpers for update/archive-style commands and standardized delete responses as `{ "id": ..., "deleted": true }`
- added CLI regression tests covering the new task/activity/job mutating JSON paths
- fixed one pre-existing `orbit-cli` clippy lint in `identity.rs` so the requested CLI lint gate passes cleanly

Strategic decisions:
- delete commands return a minimal `{ id, deleted }` payload instead of a richer status object | Rationale: the deleted resource no longer exists and this keeps the contract stable and easy to consume | Trade-offs: less metadata than archive/unarchive responses
- archive and unarchive commands emit the full updated task JSON | Rationale: the task still exists and callers often need its new status and metadata immediately | Trade-offs: slightly larger payloads

Validation:
- cargo test -p orbit-cli
- cargo clippy -p orbit-cli --no-deps -- -D warnings