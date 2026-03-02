use std::path::{Component, Path, PathBuf};

use orbit_types::OrbitError;

pub(crate) const ORBIT_ROOT_TOKEN: &str = "{{ORBIT_ROOT}}";

pub(crate) fn home_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME")
        && !home.trim().is_empty()
    {
        return Some(PathBuf::from(home));
    }
    if let Ok(profile) = std::env::var("USERPROFILE")
        && !profile.trim().is_empty()
    {
        return Some(PathBuf::from(profile));
    }
    None
}

pub(crate) fn home_dir_required(context: &str) -> Result<PathBuf, OrbitError> {
    home_dir()
        .ok_or_else(|| OrbitError::InvalidInput(format!("HOME/USERPROFILE is not set; {context}")))
}

pub(crate) fn orbit_home_root() -> PathBuf {
    home_dir()
        .map(|home| home.join(".orbit"))
        .unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".orbit")
        })
}

pub(crate) fn normalize_path(raw: Option<String>) -> Result<Option<String>, OrbitError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let path = Path::new(trimmed);
    if !path.exists() {
        return Err(OrbitError::InvalidInput(format!(
            "path does not exist: {trimmed}"
        )));
    }
    if !path.is_dir() {
        return Err(OrbitError::InvalidInput(format!(
            "path is not a directory: {trimmed}"
        )));
    }

    let canonical = path.canonicalize().map_err(|e| {
        OrbitError::InvalidInput(format!("failed to canonicalize path '{trimmed}': {e}"))
    })?;
    Ok(Some(canonical.to_string_lossy().to_string()))
}

pub(crate) fn resolve_config_path(
    raw: Option<&str>,
    default: &Path,
    base_dir: &Path,
    field_name: &str,
) -> Result<PathBuf, OrbitError> {
    let Some(raw) = raw else {
        return Ok(default.to_path_buf());
    };
    resolve_path_value(raw, base_dir, field_name)
}

pub(crate) fn resolve_path_value(
    raw: &str,
    base_dir: &Path,
    field_name: &str,
) -> Result<PathBuf, OrbitError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(OrbitError::InvalidInput(format!(
            "{field_name} must not be empty"
        )));
    }
    if value == "~" || value.starts_with("~/") {
        let home = home_dir().ok_or_else(|| {
            OrbitError::InvalidInput(
                "cannot expand '~' because HOME/USERPROFILE is not set".to_string(),
            )
        })?;
        let suffix = value.trim_start_matches("~/");
        return Ok(normalize_path_components(&home.join(suffix)));
    }
    let path = PathBuf::from(value);
    if path.is_relative() {
        return Ok(normalize_path_components(&base_dir.join(path)));
    }
    Ok(normalize_path_components(&path))
}

pub(crate) fn find_git_repo_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

pub(crate) fn normalize_path_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        if matches!(component, Component::CurDir) {
            continue;
        }
        normalized.push(component.as_os_str());
    }
    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}
