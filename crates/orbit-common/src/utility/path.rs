/// Returns true when two workspace-relative path strings refer to the same
/// path or when one path is a proper ancestor of the other on a `/` boundary.
///
/// This helper is string-only by design: it trims surrounding whitespace,
/// treats trailing slashes as equivalent, and never consults the filesystem.
pub fn workspace_relative_paths_overlap(left: &str, right: &str) -> bool {
    let Some(left) = normalize_workspace_relative_path(left) else {
        return false;
    };
    let Some(right) = normalize_workspace_relative_path(right) else {
        return false;
    };

    left == right
        || is_workspace_relative_ancestor(left, right)
        || is_workspace_relative_ancestor(right, left)
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

fn is_workspace_relative_ancestor(parent: &str, child: &str) -> bool {
    child
        .strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('/'))
}
