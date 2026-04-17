//! Workspace-level crate dependency queries.
//!
//! Derived from Cargo manifests, not the code graph: `orbit.graph.deps` asks
//! "which `orbit-*` crates does this crate directly declare as a dependency?".

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::KnowledgeError;

/// Return `{ crate_name -> [direct orbit-* deps] }` for every workspace member
/// under `repo_root`.
///
/// If `crate_filter` is supplied, only that crate's entry is returned (still in
/// a map to keep the response shape stable between filtered and unfiltered
/// queries).
pub fn crate_dependencies(
    repo_root: &Path,
    crate_filter: Option<&str>,
) -> Result<BTreeMap<String, Vec<String>>, KnowledgeError> {
    let workspace_manifest_path = repo_root.join("Cargo.toml");
    let workspace_manifest = read_manifest(&workspace_manifest_path)?;

    let members = workspace_manifest
        .get("workspace")
        .and_then(|ws| ws.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| {
            KnowledgeError::invalid_data(format!(
                "{} has no [workspace].members array",
                workspace_manifest_path.display()
            ))
        })?;

    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for member in members {
        let Some(member_path) = member.as_str() else {
            continue;
        };
        let manifest_path = repo_root.join(member_path).join("Cargo.toml");
        let Ok(manifest) = read_manifest(&manifest_path) else {
            // Skip members whose manifests fail to read/parse rather than
            // blowing up the whole response. A missing member is typically a
            // repo-state issue, not a query issue.
            continue;
        };

        let Some(name) = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        else {
            continue;
        };

        if let Some(filter) = crate_filter
            && name != filter
        {
            continue;
        }

        let mut deps = collect_orbit_deps(&manifest);
        deps.sort();
        deps.dedup();
        result.insert(name.to_string(), deps);
    }

    if let Some(filter) = crate_filter
        && !result.contains_key(filter)
    {
        return Err(KnowledgeError::invalid_data(format!(
            "crate `{filter}` is not a workspace member of {}",
            repo_root.display()
        )));
    }

    Ok(result)
}

fn collect_orbit_deps(manifest: &toml::Value) -> Vec<String> {
    let mut out = Vec::new();
    for table_key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(table) = manifest.get(table_key).and_then(|v| v.as_table()) else {
            continue;
        };
        for (dep_name, _dep_value) in table {
            if dep_name.starts_with("orbit-") {
                out.push(dep_name.clone());
            }
        }
    }
    out
}

fn read_manifest(path: &Path) -> Result<toml::Value, KnowledgeError> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| KnowledgeError::io(format!("read {}: {e}", path.display())))?;
    toml::from_str(&raw)
        .map_err(|e| KnowledgeError::invalid_data(format!("parse {}: {e}", path.display())))
}
