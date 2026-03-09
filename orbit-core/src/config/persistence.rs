use std::path::{Path, PathBuf};

use orbit_types::OrbitError;
use serde_json::{Value, json};

use crate::paths;

use super::raw::{RawPersistenceConfig, RawRuntimeConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersistenceType {
    File,
    Sqlite,
}

#[derive(Debug, Clone)]
pub(crate) struct EntityPersistenceConfig {
    pub(crate) persistence_type: PersistenceType,
    pub(crate) path: PathBuf,
    pub(crate) format: Option<String>,
}

impl EntityPersistenceConfig {
    fn to_json_value(&self) -> Value {
        json!({
            "type": match self.persistence_type {
                PersistenceType::File => "file",
                PersistenceType::Sqlite => "sqlite",
            },
            "path": self.path.to_string_lossy(),
            "format": self.format,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PersistenceConfig {
    pub(crate) job: EntityPersistenceConfig,
    pub(crate) activity: EntityPersistenceConfig,
    pub(crate) skill: EntityPersistenceConfig,
    pub(crate) task: EntityPersistenceConfig,
    pub(crate) watch: EntityPersistenceConfig,
    pub(crate) audit: EntityPersistenceConfig,
}

impl PersistenceConfig {
    pub(crate) fn default_for_data_root(data_root: &Path) -> Self {
        let sqlite_default = data_root.join("orbit.db");
        Self {
            job: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: data_root.join("jobs"),
                format: Some("yaml".to_string()),
            },
            activity: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: data_root.join("activities"),
                format: Some("yaml".to_string()),
            },
            skill: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: data_root.join("skills"),
                format: Some("md".to_string()),
            },
            task: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: data_root.join("tasks"),
                format: Some("yaml".to_string()),
            },
            watch: EntityPersistenceConfig {
                persistence_type: PersistenceType::Sqlite,
                path: sqlite_default.clone(),
                format: None,
            },
            audit: EntityPersistenceConfig {
                persistence_type: PersistenceType::Sqlite,
                path: sqlite_default,
                format: None,
            },
        }
    }

    pub(super) fn from_raw(data_root: &Path, raw: &RawRuntimeConfig) -> Result<Self, OrbitError> {
        let defaults = Self::default_for_data_root(data_root);

        Ok(Self {
            job: parse_configurable_entity(
                "job",
                raw.job.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.job,
                true,
                "yaml",
                data_root,
            )?,
            activity: parse_configurable_entity(
                "activity",
                raw.activity.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.activity,
                true,
                "yaml",
                data_root,
            )?,
            skill: parse_file_only_entity(
                "skill",
                raw.skill.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.skill,
                "md",
                data_root,
            )?,
            task: parse_file_only_entity(
                "task",
                raw.task.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.task,
                "yaml",
                data_root,
            )?,
            watch: parse_sqlite_only_entity(
                "watch",
                raw.watch.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.watch,
                data_root,
            )?,
            audit: parse_sqlite_only_entity(
                "audit",
                raw.audit.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.audit,
                data_root,
            )?,
        })
    }

    pub(crate) fn as_json_value(&self) -> Value {
        json!({
            "job": { "persistence": self.job.to_json_value() },
            "activity": { "persistence": self.activity.to_json_value() },
            "skill": { "persistence": self.skill.to_json_value() },
            "task": { "persistence": self.task.to_json_value() },
            "watch": { "persistence": self.watch.to_json_value() },
            "audit": { "persistence": self.audit.to_json_value() },
        })
    }
}

fn parse_configurable_entity(
    entity: &str,
    raw: Option<&RawPersistenceConfig>,
    defaults: &EntityPersistenceConfig,
    allow_sqlite: bool,
    required_file_format: &str,
    base_dir: &Path,
) -> Result<EntityPersistenceConfig, OrbitError> {
    let Some(raw) = raw else {
        return Ok(defaults.clone());
    };
    let persistence_type = parse_persistence_type(raw.persistence_type.as_deref(), entity)?;
    if !allow_sqlite && persistence_type == PersistenceType::Sqlite {
        return Err(OrbitError::InvalidInput(format!(
            "{entity}.persistence.type only supports 'file'"
        )));
    }

    match persistence_type {
        PersistenceType::File => {
            let format = raw
                .format
                .as_deref()
                .unwrap_or(required_file_format)
                .to_ascii_lowercase();
            if format != required_file_format {
                return Err(OrbitError::InvalidInput(format!(
                    "{entity}.persistence.format must be '{required_file_format}' for file persistence"
                )));
            }
            Ok(EntityPersistenceConfig {
                persistence_type,
                path: paths::resolve_config_path(
                    raw.path.as_deref(),
                    &defaults.path,
                    base_dir,
                    "persistence.path",
                )?,
                format: Some(format),
            })
        }
        PersistenceType::Sqlite => {
            if raw.format.is_some() {
                return Err(OrbitError::InvalidInput(format!(
                    "{entity}.persistence.format is not supported for sqlite persistence"
                )));
            }
            Ok(EntityPersistenceConfig {
                persistence_type,
                path: paths::resolve_config_path(
                    raw.path.as_deref(),
                    &defaults.path,
                    base_dir,
                    "persistence.path",
                )?,
                format: None,
            })
        }
    }
}

fn parse_file_only_entity(
    entity: &str,
    raw: Option<&RawPersistenceConfig>,
    defaults: &EntityPersistenceConfig,
    required_file_format: &str,
    base_dir: &Path,
) -> Result<EntityPersistenceConfig, OrbitError> {
    parse_configurable_entity(entity, raw, defaults, false, required_file_format, base_dir)
}

fn parse_sqlite_only_entity(
    entity: &str,
    raw: Option<&RawPersistenceConfig>,
    defaults: &EntityPersistenceConfig,
    base_dir: &Path,
) -> Result<EntityPersistenceConfig, OrbitError> {
    let Some(raw) = raw else {
        return Ok(defaults.clone());
    };
    let persistence_type = match raw.persistence_type.as_deref() {
        None => PersistenceType::Sqlite,
        Some(value) => parse_persistence_type(Some(value), entity)?,
    };
    if persistence_type != PersistenceType::Sqlite {
        return Err(OrbitError::InvalidInput(format!(
            "{entity}.persistence.type only supports 'sqlite'"
        )));
    }
    if raw.format.is_some() {
        return Err(OrbitError::InvalidInput(format!(
            "{entity}.persistence.format is not supported for sqlite persistence"
        )));
    }

    Ok(EntityPersistenceConfig {
        persistence_type,
        path: paths::resolve_config_path(
            raw.path.as_deref(),
            &defaults.path,
            base_dir,
            "persistence.path",
        )?,
        format: None,
    })
}

fn parse_persistence_type(raw: Option<&str>, entity: &str) -> Result<PersistenceType, OrbitError> {
    let value = raw.unwrap_or("file").trim().to_ascii_lowercase();
    match value.as_str() {
        "file" => Ok(PersistenceType::File),
        "sqlite" => Ok(PersistenceType::Sqlite),
        other => Err(OrbitError::InvalidInput(format!(
            "{entity}.persistence.type must be 'file' or 'sqlite' (got '{other}')"
        ))),
    }
}
