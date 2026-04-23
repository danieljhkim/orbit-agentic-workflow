use std::fs;
use std::path::Path;

use orbit_common::types::OrbitError;
use orbit_common::utility::fs::atomic_write_text_volatile as write_atomic;

pub(crate) fn read_yaml_with<T: serde::de::DeserializeOwned, F>(
    path: &Path,
    invalid: F,
) -> Result<T, OrbitError>
where
    F: FnOnce(&Path, serde_yaml::Error) -> OrbitError,
{
    let raw = fs::read_to_string(path).map_err(|e| OrbitError::Io(e.to_string()))?;
    serde_yaml::from_str(&raw).map_err(|err| invalid(path, err))
}

pub(crate) fn write_yaml_atomic_with<T, F>(
    path: &Path,
    value: &T,
    serialize: F,
) -> Result<(), OrbitError>
where
    F: FnOnce(&T) -> Result<String, OrbitError>,
{
    let yaml = serialize(value)?;
    write_atomic(path, &yaml).map_err(Into::into)
}
