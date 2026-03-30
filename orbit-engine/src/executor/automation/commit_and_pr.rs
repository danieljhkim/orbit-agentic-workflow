use orbit_types::OrbitError;
use serde_json::Value;

use crate::context::{RuntimeHost, TaskHost};

/// Combined automation: commit batch changes, then open a PR.
///
/// Calls the existing `commit_batch_changes` and `open_batch_pr` sequentially,
/// merging their JSON outputs into a single response.
pub(super) fn commit_and_open_batch_pr<H: RuntimeHost + TaskHost + Sync + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let mut commit_result = super::commit::commit_batch_changes(host, input)?;
    let pr_result = super::pr::open_batch_pr(host, input)?;

    // Merge pr_result fields into commit_result so the caller gets a union of both outputs.
    if let (Some(base), Some(overlay)) = (commit_result.as_object_mut(), pr_result.as_object()) {
        for (key, value) in overlay {
            base.insert(key.clone(), value.clone());
        }
    }

    Ok(commit_result)
}
