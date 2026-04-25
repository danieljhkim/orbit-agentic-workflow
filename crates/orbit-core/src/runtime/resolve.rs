use std::fs;
use std::path::{Path, PathBuf};

use orbit_common::types::OrbitError;
use serde::Deserialize;

use crate::paths;
use crate::workspace_registry;

/// Returns the global orbit root at `~/.orbit/`.
pub(crate) fn resolve_global_root() -> Result<PathBuf, OrbitError> {
    workspace_registry::global_orbit_dir()
}

/// Resolves the `.orbit` data root using the full resolution chain.
///
/// Resolution order:
/// 1. `--root` flag (escape hatch)
/// 2. `ORBIT_ROOT` env (escape hatch)
/// 3. `path_overrides` in global registry (longest prefix match from cwd)
/// 4. Walk up from cwd to find first `.orbit/` directory
/// 5. Legacy: git repo root (for repos without `.orbit/` directory yet)
/// 6. Fallback: `<cwd>/.orbit`
pub(crate) fn resolve_initialize_data_root(
    cwd: &Path,
    root_override: Option<&Path>,
) -> Result<PathBuf, OrbitError> {
    resolve_data_root(cwd, root_override, ExplicitRootMode::RequireInitialized)
}

/// Resolves the `.orbit` data root for commands that are allowed to create it.
pub(crate) fn resolve_bootstrap_data_root(
    cwd: &Path,
    root_override: Option<&Path>,
) -> Result<PathBuf, OrbitError> {
    resolve_data_root(cwd, root_override, ExplicitRootMode::AllowUninitialized)
}

fn resolve_data_root(
    cwd: &Path,
    root_override: Option<&Path>,
    explicit_root_mode: ExplicitRootMode,
) -> Result<PathBuf, OrbitError> {
    // 1. --root flag (escape hatch)
    if let Some(root) = root_override {
        return resolve_explicit_root_path_value(&root.to_string_lossy(), cwd, explicit_root_mode);
    }

    // 2. ORBIT_ROOT env (escape hatch)
    if let Ok(explicit) = std::env::var("ORBIT_ROOT")
        && !explicit.trim().is_empty()
    {
        return resolve_explicit_root_path_value(&explicit, cwd, explicit_root_mode);
    }

    // 3. path_overrides in global registry (longest prefix match)
    if let Some(ws) = resolve_from_path_override(cwd) {
        return Ok(ws);
    }

    // 4. Walk up from cwd to find first .orbit/ directory
    if let Some(orbit_dir) = find_orbit_dir_walk_up(cwd) {
        // Check if this .orbit has a config.toml with a root redirect
        let config_path = orbit_dir.join("config.toml");
        if config_path.exists()
            && let Some(configured_root) = configured_root_from_config(&config_path)?
        {
            return Ok(configured_root);
        }
        return Ok(orbit_dir);
    }

    // 5. Legacy: git repo root (for repos without .orbit/ directory yet)
    if let Some(repo_root) = paths::find_git_repo_root(cwd) {
        return Ok(repo_root.join(".orbit"));
    }

    // 6. Fallback: <cwd>/.orbit
    Ok(paths::cwd_orbit_root(cwd))
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ExplicitRootMode {
    AllowUninitialized,
    RequireInitialized,
}

/// Checks path_overrides in the global registry for a matching workspace.
fn resolve_from_path_override(cwd: &Path) -> Option<PathBuf> {
    let registry = workspace_registry::load_registry().ok()?;
    let ws = workspace_registry::find_workspace_by_path(&registry, cwd)?;
    Some(ws.orbit_dir.clone())
}

/// Walks up the directory tree from `start` looking for the first `.orbit/` directory.
fn find_orbit_dir_walk_up(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        let candidate = current.join(".orbit");
        if candidate.is_dir() {
            return Some(candidate);
        }
        current = current.parent()?;
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RootField {
    String(String),
    Table { path: String },
}

#[derive(Debug, Deserialize)]
struct RootOnlyConfig {
    root: Option<RootField>,
}

fn configured_root_from_config(config_path: &Path) -> Result<Option<PathBuf>, OrbitError> {
    let raw = fs::read_to_string(config_path).map_err(|err| {
        OrbitError::Io(format!(
            "failed to read runtime config '{}': {err}",
            config_path.display()
        ))
    })?;
    let parsed = toml::from_str::<RootOnlyConfig>(&raw).map_err(|err| {
        OrbitError::InvalidInput(format!(
            "invalid runtime config '{}': {err}",
            config_path.display()
        ))
    })?;
    let Some(root_value) = parsed.root else {
        return Ok(None);
    };
    let root_value = match root_value {
        RootField::String(value) => value,
        RootField::Table { path } => path,
    };
    let base = config_path.parent().ok_or_else(|| {
        OrbitError::InvalidInput(format!(
            "invalid config path without parent: {}",
            config_path.display()
        ))
    })?;
    Ok(Some(resolve_root_path_value(&root_value, base)?))
}

fn resolve_explicit_root_path_value(
    raw: &str,
    base_dir: &Path,
    mode: ExplicitRootMode,
) -> Result<PathBuf, OrbitError> {
    let root = resolve_root_path_value(raw, base_dir)?;
    match mode {
        ExplicitRootMode::AllowUninitialized => Ok(root),
        ExplicitRootMode::RequireInitialized => resolve_initialized_root(root),
    }
}

fn resolve_initialized_root(root: PathBuf) -> Result<PathBuf, OrbitError> {
    let child_orbit = root.join(".orbit");
    if is_initialized_orbit_root(&child_orbit) {
        return Ok(child_orbit);
    }

    if is_initialized_orbit_root(&root) {
        return Ok(root);
    }

    Err(OrbitError::InvalidInput(format!(
        "{} is not an Orbit workspace; run `orbit workspace init` first or pass `--root <path/to/.orbit>`",
        root.display()
    )))
}

fn is_initialized_orbit_root(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    if path.join("config.toml").is_file() {
        return true;
    }

    path.join("resources").is_dir() && path.join("tasks").is_dir() && path.join("state").is_dir()
}

fn resolve_root_path_value(raw: &str, base_dir: &Path) -> Result<PathBuf, OrbitError> {
    paths::resolve_path_value(raw, base_dir, "root path")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn explicit_root_with_initialized_child_orbit_resolves_to_child() {
        let repo = tempdir().expect("repo tempdir");
        let orbit_root = repo.path().join(".orbit");
        seed_initialized_workspace_root(&orbit_root);

        let resolved =
            resolve_initialize_data_root(repo.path(), Some(repo.path())).expect("resolve root");

        assert_eq!(resolved, orbit_root);
    }

    #[test]
    fn explicit_root_prefers_initialized_child_orbit_over_polluted_repo_root() {
        let repo = tempdir().expect("repo tempdir");
        let orbit_root = repo.path().join(".orbit");
        seed_initialized_workspace_root(&orbit_root);
        fs::write(repo.path().join("config.toml"), "polluted = true\n")
            .expect("write root pollution");

        let resolved =
            resolve_initialize_data_root(repo.path(), Some(repo.path())).expect("resolve root");

        assert_eq!(resolved, orbit_root);
    }

    #[test]
    fn explicit_root_with_uninitialized_directory_returns_invalid_input_without_layout() {
        let parent = tempdir().expect("parent tempdir");
        let root = parent.path().join("not-an-orbit-root");
        fs::create_dir_all(&root).expect("create uninitialized root");

        let err = resolve_initialize_data_root(parent.path(), Some(&root))
            .expect_err("uninitialized root should fail");

        assert!(matches!(
            err,
            OrbitError::InvalidInput(message) if message.contains("not an Orbit workspace")
        ));
        assert!(!root.join(".orbit").exists());
        assert!(!root.join("resources").exists());
        assert!(!root.join("tasks").exists());
        assert!(!root.join("state").exists());
    }

    #[test]
    fn explicit_root_with_initialized_orbit_root_resolves_as_is() {
        let repo = tempdir().expect("repo tempdir");
        let orbit_root = repo.path().join(".orbit");
        seed_initialized_workspace_root(&orbit_root);

        let resolved =
            resolve_initialize_data_root(repo.path(), Some(&orbit_root)).expect("resolve root");

        assert_eq!(resolved, orbit_root);
    }

    #[test]
    fn bootstrap_root_allows_uninitialized_path_without_creating_it() {
        let parent = tempdir().expect("parent tempdir");
        let root = parent.path().join("new-orbit-root");

        let resolved =
            resolve_bootstrap_data_root(parent.path(), Some(&root)).expect("resolve root");

        assert_eq!(resolved, root);
        assert!(!root.exists());
    }

    fn seed_initialized_workspace_root(path: &Path) {
        fs::create_dir_all(path.join("resources")).expect("create resources");
        fs::create_dir_all(path.join("tasks")).expect("create tasks");
        fs::create_dir_all(path.join("state")).expect("create state");
    }
}
