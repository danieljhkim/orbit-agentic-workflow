use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use orbit_types::OrbitError;
use serde::Deserialize;
use serde_json::{Value, json};

const DEFAULT_ENV_INHERIT: bool = false;
const DEFAULT_ENV_PASS: [&str; 3] = ["HOME", "PATH", "CODEX_HOME"];

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    pub(crate) execution_env: ExecutionEnvPolicy,
    pub(crate) persistence: PersistenceConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        let data_root = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".orbit");
        Self::default_for_data_root(&data_root)
    }
}

impl RuntimeConfig {
    pub(crate) fn default_for_data_root(data_root: &Path) -> Self {
        Self {
            execution_env: ExecutionEnvPolicy::default(),
            persistence: PersistenceConfig::default_for_data_root(data_root),
        }
    }

    pub(crate) fn load_from_data_root(data_root: &Path) -> Result<Self, OrbitError> {
        let config_path = data_root.join("config.toml");
        if !config_path.exists() {
            return Ok(Self::default_for_data_root(data_root));
        }

        let raw = fs::read_to_string(&config_path).map_err(|err| {
            OrbitError::Io(format!(
                "failed to read runtime config '{}': {err}",
                config_path.display()
            ))
        })?;
        let parsed = toml::from_str::<RawRuntimeConfig>(&raw).map_err(|err| {
            OrbitError::InvalidInput(format!(
                "invalid runtime config '{}': {err}",
                config_path.display()
            ))
        })?;

        Ok(Self {
            execution_env: ExecutionEnvPolicy::from_raw(
                parsed.execution.clone().and_then(|v| v.env),
            )?,
            persistence: PersistenceConfig::from_raw(data_root, &parsed)?,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExecutionEnvPolicy {
    inherit: bool,
    pass: Vec<String>,
}

impl Default for ExecutionEnvPolicy {
    fn default() -> Self {
        Self {
            inherit: DEFAULT_ENV_INHERIT,
            pass: default_pass_list(),
        }
    }
}

impl ExecutionEnvPolicy {
    fn from_raw(raw: Option<RawExecutionEnvConfig>) -> Result<Self, OrbitError> {
        match raw {
            Some(raw) => {
                let inherit = raw.inherit.unwrap_or(DEFAULT_ENV_INHERIT);
                let pass = normalize_pass_list(raw.pass.unwrap_or_else(default_pass_list))?;
                Ok(Self { inherit, pass })
            }
            None => Ok(Self::default()),
        }
    }

    pub(crate) fn inherit(&self) -> bool {
        self.inherit
    }

    pub(crate) fn pass(&self) -> &[String] {
        &self.pass
    }

    pub(crate) fn hydrated_allowlist_env(&self) -> Vec<(String, String)> {
        self.pass
            .iter()
            .filter_map(|name| std::env::var(name).ok().map(|value| (name.clone(), value)))
            .collect()
    }

    pub(crate) fn missing_required_for_provider(&self, provider: &str) -> Vec<String> {
        required_env_vars_for_provider(provider)
            .iter()
            .copied()
            .filter(|name| !self.is_required_var_available(name))
            .map(ToString::to_string)
            .collect()
    }

    fn is_required_var_available(&self, name: &str) -> bool {
        if self.inherit {
            return std::env::var(name).is_ok();
        }
        self.pass.iter().any(|candidate| candidate == name) && std::env::var(name).is_ok()
    }
}

fn default_pass_list() -> Vec<String> {
    DEFAULT_ENV_PASS.iter().map(ToString::to_string).collect()
}

fn normalize_pass_list(pass: Vec<String>) -> Result<Vec<String>, OrbitError> {
    let mut normalized = BTreeSet::new();
    for entry in pass {
        let value = entry.trim();
        if value.is_empty() {
            return Err(OrbitError::InvalidInput(
                "execution.env.pass must not contain empty variable names".to_string(),
            ));
        }
        if !is_valid_env_var_name(value) {
            return Err(OrbitError::InvalidInput(format!(
                "execution.env.pass contains invalid variable name '{value}'"
            )));
        }
        normalized.insert(value.to_string());
    }
    Ok(normalized.into_iter().collect())
}

fn is_valid_env_var_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn required_env_vars_for_provider(provider: &str) -> &'static [&'static str] {
    match provider {
        "codex" => &["HOME", "PATH"],
        "claude" => &["HOME", "PATH"],
        _ => &[],
    }
}

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
    pub(crate) work: EntityPersistenceConfig,
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
            work: EntityPersistenceConfig {
                persistence_type: PersistenceType::File,
                path: data_root.join("works"),
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

    fn from_raw(data_root: &Path, raw: &RawRuntimeConfig) -> Result<Self, OrbitError> {
        let defaults = Self::default_for_data_root(data_root);

        Ok(Self {
            job: parse_configurable_entity(
                "job",
                raw.job.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.job,
                true,
                "yaml",
            )?,
            work: parse_configurable_entity(
                "work",
                raw.work.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.work,
                true,
                "yaml",
            )?,
            skill: parse_file_only_entity(
                "skill",
                raw.skill.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.skill,
                "md",
            )?,
            task: parse_file_only_entity(
                "task",
                raw.task.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.task,
                "yaml",
            )?,
            watch: parse_sqlite_only_entity(
                "watch",
                raw.watch.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.watch,
            )?,
            audit: parse_sqlite_only_entity(
                "audit",
                raw.audit.as_ref().and_then(|v| v.persistence.as_ref()),
                &defaults.audit,
            )?,
        })
    }

    pub(crate) fn as_json_value(&self) -> Value {
        json!({
            "job": { "persistence": self.job.to_json_value() },
            "work": { "persistence": self.work.to_json_value() },
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
                path: resolve_path(raw.path.as_deref(), &defaults.path)?,
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
                path: resolve_path(raw.path.as_deref(), &defaults.path)?,
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
) -> Result<EntityPersistenceConfig, OrbitError> {
    parse_configurable_entity(entity, raw, defaults, false, required_file_format)
}

fn parse_sqlite_only_entity(
    entity: &str,
    raw: Option<&RawPersistenceConfig>,
    defaults: &EntityPersistenceConfig,
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
        path: resolve_path(raw.path.as_deref(), &defaults.path)?,
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

fn resolve_path(raw: Option<&str>, default: &Path) -> Result<PathBuf, OrbitError> {
    let Some(raw) = raw else {
        return Ok(default.to_path_buf());
    };
    let value = raw.trim();
    if value.is_empty() {
        return Err(OrbitError::InvalidInput(
            "persistence.path must not be empty".to_string(),
        ));
    }
    if value == "~" || value.starts_with("~/") {
        let home = std::env::var("HOME").map_err(|_| {
            OrbitError::InvalidInput("cannot expand '~' because HOME is not set".to_string())
        })?;
        let suffix = value.trim_start_matches("~/");
        return Ok(PathBuf::from(home).join(suffix));
    }
    let path = PathBuf::from(value);
    if path.is_relative() {
        return Ok(std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path));
    }
    Ok(path)
}

#[derive(Debug, Clone, Deserialize)]
struct RawRuntimeConfig {
    execution: Option<RawExecutionConfig>,
    job: Option<RawEntitySection>,
    work: Option<RawEntitySection>,
    skill: Option<RawEntitySection>,
    task: Option<RawEntitySection>,
    watch: Option<RawEntitySection>,
    audit: Option<RawEntitySection>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawExecutionConfig {
    env: Option<RawExecutionEnvConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawExecutionEnvConfig {
    inherit: Option<bool>,
    pass: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawEntitySection {
    persistence: Option<RawPersistenceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPersistenceConfig {
    #[serde(rename = "type")]
    persistence_type: Option<String>,
    #[serde(alias = "ppath")]
    path: Option<String>,
    format: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{PersistenceConfig, normalize_pass_list};
    use std::path::Path;

    #[test]
    fn normalize_pass_list_rejects_invalid_identifiers() {
        let err = normalize_pass_list(vec!["1INVALID".to_string()]).expect_err("must fail");
        assert!(err.to_string().contains("invalid variable name"));
    }

    #[test]
    fn normalize_pass_list_dedupes_and_sorts() {
        let values = normalize_pass_list(vec![
            "PATH".to_string(),
            "HOME".to_string(),
            "PATH".to_string(),
        ])
        .expect("normalize");
        assert_eq!(values, vec!["HOME".to_string(), "PATH".to_string()]);
    }

    #[test]
    fn persistence_defaults_to_file_for_jobs_and_works() {
        let config = PersistenceConfig::default_for_data_root(Path::new("/tmp/orbit"));
        assert_eq!(config.job.path, std::path::PathBuf::from("/tmp/orbit/jobs"));
        assert_eq!(
            config.work.path,
            std::path::PathBuf::from("/tmp/orbit/works")
        );
        assert_eq!(config.job.format.as_deref(), Some("yaml"));
        assert_eq!(config.work.format.as_deref(), Some("yaml"));
    }
}
