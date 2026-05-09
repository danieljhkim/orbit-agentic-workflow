use std::fs;
use std::path::Path;

use orbit_core::OrbitError;
use toml::{Table as TomlTable, Value as TomlValue};

pub(in crate::command::mcp::setup) fn load_toml_table(
    path: &Path,
) -> Result<TomlTable, OrbitError> {
    if !path.exists() {
        return Ok(TomlTable::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|err| OrbitError::Io(format!("failed to read '{}': {err}", path.display())))?;
    if raw.trim().is_empty() {
        return Ok(TomlTable::new());
    }

    let value: TomlValue = toml::from_str(&raw).map_err(|err| {
        OrbitError::InvalidInput(format!("invalid TOML '{}': {err}", path.display()))
    })?;
    value.as_table().cloned().ok_or_else(|| {
        OrbitError::InvalidInput(format!(
            "expected top-level TOML table in '{}'",
            path.display()
        ))
    })
}

pub(in crate::command::mcp::setup) fn write_toml_table(
    path: &Path,
    root: &TomlTable,
) -> Result<(), OrbitError> {
    let parent = path.parent().ok_or_else(|| {
        OrbitError::InvalidInput(format!("path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| OrbitError::Io(format!("failed to create '{}': {err}", parent.display())))?;
    let rendered = toml::to_string_pretty(&TomlValue::Table(root.clone())).map_err(|err| {
        OrbitError::Execution(format!("serialize TOML '{}': {err}", path.display()))
    })?;
    fs::write(path, rendered)
        .map_err(|err| OrbitError::Io(format!("failed to write '{}': {err}", path.display())))
}

pub(in crate::command::mcp::setup) fn write_or_remove_toml_table(
    path: &Path,
    root: &TomlTable,
) -> Result<(), OrbitError> {
    if root.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|err| {
                OrbitError::Io(format!("failed to remove '{}': {err}", path.display()))
            })?;
        }
        return Ok(());
    }
    write_toml_table(path, root)
}

pub(in crate::command::mcp::setup) fn ensure_toml_table<'a>(
    root: &'a mut TomlTable,
    key: &str,
) -> Result<&'a mut TomlTable, OrbitError> {
    let value = root
        .entry(key.to_string())
        .or_insert_with(|| TomlValue::Table(TomlTable::new()));
    value
        .as_table_mut()
        .ok_or_else(|| OrbitError::InvalidInput(format!("expected '{key}' to be a TOML table")))
}
