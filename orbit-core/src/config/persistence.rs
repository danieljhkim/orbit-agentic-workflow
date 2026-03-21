use std::path::{Path, PathBuf};

use orbit_store::{ResolvedScope, ScopeResolution};
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

/// Carries a global path, optional workspace path, and the resolution strategy
/// for a single artifact type. Call `resolve()` to get the effective store path(s).
#[derive(Debug, Clone)]
pub(crate) struct ArtifactScope {
    pub(crate) global_path: PathBuf,
    pub(crate) workspace_path: Option<PathBuf>,
    pub(crate) resolution: ScopeResolution,
}

impl ArtifactScope {
    pub(crate) fn resolve(&self) -> ResolvedScope {
        match self.resolution {
            ScopeResolution::GlobalOnly => ResolvedScope::Single(self.global_path.clone()),
            ScopeResolution::WorkspaceOnly => ResolvedScope::Single(
                self.workspace_path
                    .clone()
                    .unwrap_or_else(|| self.global_path.clone()),
            ),
            ScopeResolution::WorkspaceReplaces => match &self.workspace_path {
                Some(ws) if ws.is_dir() => ResolvedScope::Single(ws.clone()),
                _ => ResolvedScope::Single(self.global_path.clone()),
            },
            ScopeResolution::MergeByKey => match &self.workspace_path {
                Some(ws) if ws.is_dir() => ResolvedScope::Layered {
                    global: self.global_path.clone(),
                    workspace: ws.clone(),
                },
                _ => ResolvedScope::Single(self.global_path.clone()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PersistenceConfig {
    pub(crate) job: ArtifactScope,
    pub(crate) activity: ArtifactScope,
    pub(crate) skill: ArtifactScope,
    pub(crate) task: ArtifactScope,
    pub(crate) audit: ArtifactScope,
    // Format metadata for file-based entities (used for config parsing/validation and JSON output)
    pub(crate) job_format: EntityPersistenceConfig,
    pub(crate) activity_format: EntityPersistenceConfig,
    pub(crate) audit_format: EntityPersistenceConfig,
}

impl PersistenceConfig {
    pub(crate) fn default_for_data_root(data_root: &Path) -> Self {
        Self::default_for_roots(data_root, data_root)
    }

    /// Two-root defaults. This is the **single source of truth** for which
    /// artifact gets which scope resolution strategy.
    pub(crate) fn default_for_roots(global_root: &Path, workspace_root: &Path) -> Self {
        let ws = if global_root == workspace_root {
            None
        } else {
            Some(workspace_root)
        };

        Self {
            task: ArtifactScope {
                global_path: global_root.join("tasks"),
                workspace_path: ws.map(|p| p.join("tasks")),
                resolution: ScopeResolution::WorkspaceOnly,
            },
            activity: ArtifactScope {
                global_path: global_root.join("activities"),
                workspace_path: ws.map(|p| p.join("activities")),
                resolution: ScopeResolution::MergeByKey,
            },
            job: ArtifactScope {
                global_path: global_root.join("jobs"),
                workspace_path: ws.map(|p| p.join("jobs")),
                resolution: ScopeResolution::MergeByKey,
            },
            skill: ArtifactScope {
                global_path: global_root.join("skills"),
                workspace_path: ws.map(|p| p.join("skills")),
                resolution: ScopeResolution::WorkspaceReplaces,
            },
            audit: ArtifactScope {
                global_path: global_root.join("orbit.db"),
                workspace_path: None,
                resolution: ScopeResolution::GlobalOnly,
            },
            // Format metadata (used for config validation and JSON output)
            job_format: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: global_root.join("jobs"),
                format: Some("yaml".to_string()),
            },
            activity_format: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: global_root.join("activities"),
                format: Some("yaml".to_string()),
            },
            audit_format: EntityPersistenceConfig {
                persistence_type: PersistenceType::Sqlite,
                path: global_root.join("orbit.db"),
                format: None,
            },
        }
    }

    #[allow(dead_code)]
    pub(super) fn from_raw(data_root: &Path, raw: &RawRuntimeConfig) -> Result<Self, OrbitError> {
        Self::from_raw_layered(data_root, data_root, data_root, raw)
    }

    /// Parse persistence config with two-root awareness.
    /// `global_root`/`workspace_root` set the defaults; `config_root` is used as
    /// the base directory for resolving relative paths in the config file.
    pub(super) fn from_raw_layered(
        global_root: &Path,
        workspace_root: &Path,
        config_root: &Path,
        raw: &RawRuntimeConfig,
    ) -> Result<Self, OrbitError> {
        let defaults = Self::default_for_roots(global_root, workspace_root);
        if raw.watch.is_some() {
            return Err(OrbitError::InvalidInput(
                "watch config is no longer supported; remove the [watch] section from the runtime config file (.orbit/config.toml in a repo-local workspace, or <data_root>/config.toml)"
                    .to_string(),
            ));
        }

        let skill_path = resolve_path_only_entity(
            raw.skill.as_ref().and_then(|v| v.persistence.as_ref()),
            &defaults.skill.global_path,
            config_root,
        )?;

        let task_path = resolve_path_only_entity(
            raw.task.as_ref().and_then(|v| v.persistence.as_ref()),
            &defaults.task.global_path,
            config_root,
        )?;

        let job_format = parse_configurable_entity(
            "job",
            raw.job.as_ref().and_then(|v| v.persistence.as_ref()),
            &defaults.job_format,
            false,
            "yaml",
            config_root,
        )?;
        let activity_format = parse_configurable_entity(
            "activity",
            raw.activity.as_ref().and_then(|v| v.persistence.as_ref()),
            &defaults.activity_format,
            false,
            "yaml",
            config_root,
        )?;
        let audit_format = parse_sqlite_only_entity(
            "audit",
            raw.audit.as_ref().and_then(|v| v.persistence.as_ref()),
            &defaults.audit_format,
            config_root,
        )?;

        // Rebuild scopes with config-overridden paths while preserving resolution strategies
        Ok(Self {
            task: ArtifactScope {
                global_path: task_path,
                ..defaults.task
            },
            activity: ArtifactScope {
                global_path: activity_format.path.clone(),
                ..defaults.activity
            },
            job: ArtifactScope {
                global_path: job_format.path.clone(),
                ..defaults.job
            },
            skill: ArtifactScope {
                global_path: skill_path,
                ..defaults.skill
            },
            audit: ArtifactScope {
                global_path: audit_format.path.clone(),
                ..defaults.audit
            },
            job_format,
            activity_format,
            audit_format,
        })
    }

    pub(crate) fn as_json_value(&self) -> Value {
        json!({
            "job": { "persistence": self.job_format.to_json_value() },
            "activity": { "persistence": self.activity_format.to_json_value() },
            "skill": { "path": self.skill.global_path.to_string_lossy() },
            "task": { "path": self.task.global_path.to_string_lossy() },
            "audit": { "persistence": self.audit_format.to_json_value() },
        })
    }
}

fn resolve_path_only_entity(
    raw: Option<&RawPersistenceConfig>,
    default_path: &Path,
    base_dir: &Path,
) -> Result<PathBuf, OrbitError> {
    let Some(raw) = raw else {
        return Ok(default_path.to_path_buf());
    };
    paths::resolve_config_path(
        raw.path.as_deref(),
        default_path,
        base_dir,
        "persistence.path",
    )
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
