use super::selector::overlaps;

/// Returns true when two task context scopes overlap on the same filesystem
/// anchor or on an ancestor/descendant boundary.
///
/// This helper accepts both legacy raw paths and canonical selector strings
/// and delegates to the shared selector overlap semantics.
pub fn workspace_relative_paths_overlap(left: &str, right: &str) -> bool {
    overlaps(left, right)
}

pub fn normalize_workspace_relative_path(path: &str) -> Option<&str> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.trim_end_matches('/');
    if normalized.is_empty() {
        return None;
    }

    Some(normalized)
}
